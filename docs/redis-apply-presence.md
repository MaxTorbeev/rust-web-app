# Redis: apply_presence

Статус: основной `apply_presence.lua` и `RedisChannelStore::apply_presence`
реализованы. Один Lua-вызов фиксирует batch, ledger и outbox; Rust разбирает
результат через `decode_presence`. Подключён через `PresenceStore` к runtime.
Live Redis и двухнодовый тест проверяют commit, replay, fingerprint conflict,
сохранение JSON payload и доставку через outbox.

## Вызов из Rust

`RedisChannelStore` принимает `PresenceLedgerPolicy` при создании (из настроек
приложения). Capacity нормализуется до значения не меньше 1, как в memory store;
retention используется при disconnect. Новые ограничения размера batch
или числа участников не вводятся.

Rust проверяет принадлежность канала приложению и actor текущему запуску ноды,
сериализует payload и вызывает `scripts::APPLY_PRESENCE`.

| KEYS | Значение |
|---|---|
| 1 | node lease |
| 2 | generation deadlines |
| 3 | channel state |
| 4 | channel attachments |
| 5 | channel members |
| 6 | connection state |
| 7 | connection channels |
| 8 | connection members |
| 9 | generation connections |
| 10 | outbox |
| 11 | HASH текущей операции (`connection_operation`) |
| 12 | connection operation order |

| ARGV | Значение |
|---|---|
| 1 | Полное значение lease token |
| 2 | Generation G |
| 3, 4, 5 | Сегменты A, C, K |
| 6 | JSON ChannelKey |
| 7 | Исходный connection ID |
| 8 | JSON NodeInstance команды |
| 9 | JSON items из encode_presence_items |
| 10 | Serial S: 20 десятичных цифр |
| 11 | Fingerprint запроса |
| 12 | Исходный event ID |
| 13 | Capacity ledger |

Все ключи и аргументы создаёт типизированный Rust-адаптер. Скрипт не является
публичным API для произвольных KEYS/ARGV. Ключи вытесняемых HASH вычисляются
из префикса KEYS[11] и сохранённых serial; текущая схема рассчитана на standalone
Redis, не на Redis Cluster.

Ответ: `{"committed", outbox_fields}`, `{"rejected", code[, client_id]}`,
`{"replayed", record_fields}` либо ошибка хранилища `{0, code, message}`.
При неопределённом результате запроса повторяют исходную команду с тем же
serial/fingerprint. Store не генерирует новый event ID и не публикует событие
отдельно от outbox. `request_time` не используется: время назначает Redis.

## Формат участника

JSON кодирует Rust в `MemberPayload`: `connectionId`, `clientId`, `nodeInstance`,
`dataJson`, `lastMessageId`. `dataJson` — строка с сериализованными данными либо
JSON null при отсутствии данных. Строка `"null"` сохраняет явно переданный null.
Lua может читать identity, но не разбирает вложенный пользовательский JSON.

HASH `channel_members` содержит три поля на участника:

- `K.U` — исходный JSON `MemberPayload` из Rust.
- `K.U:revision` — Presence revision, десятичная строка.
- `K.U:updated_at` — время Redis TIME, десятичная строка миллисекунд.

Все три поля и обратная ссылка `C.U` записываются/удаляются одним transition.
Revision и timestamp назначает Redis; JSON payload остаётся неизменным.
Attach/snapshot используют `read_presence_members`: одно HGETALL, группировка
в `field/{payload, revision, timestamp}`, проверка отсутствующих и лишних metadata.
Rust собирает `PresenceMember` из этой тройки и разбирает `dataJson` через serde.

## Повтор команды и ledger

`protocol::operation_serial` кодирует `msg_serial` как 20 десятичных цифр.
Score всех записей order ZSET равен нулю; serial сравниваются строками.
Это сохраняет весь диапазон `u64` без `tonumber`.

Каждая операция имеет отдельный HASH `app.A.connection.K.operations.S`:
`RedisKeys::connection_operation(app, connection, serial)`. Метод
`connection_operations` возвращает только общий префикс, без данных по этому ключу.

Поля HASH: `fingerprint`, `result` (`committed`/`rejected`) и:

- для committed — все поля исходного outbox `presence.v2`;
- для rejected — `code` и `client_id` для `clientIdNotAllowed`.

`prepare_presence_operation(fingerprint, outcome)` готовит плоские пары для HASH,
без JSON-кодирования. Outcome имеет вид `{"committed", outbox_pairs}` либо
`{"rejected", code[, client_id]}`. Инфраструктурные ошибки в ledger не записываются.

`lookup_presence_operation(operation_key, order_key, serial, fingerprint,
highest_serial, closed)` вызывается после проверки lease и поколения connection
state, до проверки attachment. Читает HASH операции через HGETALL:

1. Известный serial с прежним fingerprint → `{"replayed", record_fields}`.
2. Известный serial с другим fingerprint → `{"rejected", "conflictingReplay"}`.
3. Неизвестный serial не больше `highest_serial` и меньше минимального
   сохранённого (либо окно пусто) → `{"rejected", "staleOperation"}`.
4. Неизвестная операция закрытого соединения → `{"rejected", "connectionClosed"}`.
5. Иначе `nil`: выполнение новой операции продолжается.

Пропуски внутри открытого окна допустимы; известный результат воспроизводится
после disconnect. Три отказа lookup не сохраняются как новые операции.
Повреждённая запись возвращается вторым значением `{0, "corrupt_state", message}`.

Основной transition атомарно записывает HASH результата и serial в ZSET,
обновляет highest serial и удаляет вытесненные HASH вместе с записями ZSET.
Сохраняются старшие capacity serial, включая случай уменьшения capacity и
новой операции внутри окна. План вытеснения готовится до первой записи.
Соответствие записи текущей операции её ZSET index проверяется до lookup.
Доменный отказ тоже создаёт открытый connection state и generation index,
если это первая операция соединения, даже без attachment.
При закрытии соединения каждый сохранённый HASH получает тот же абсолютный
срок удаления, что connection state и order ZSET: это выполняет
[disconnect](redis-disconnect.md). При закрытии connection удаляется из generation
index, но известные операции доступны для replay до истечения retention. Replay возвращает полные данные исходного события, а не ссылку
на stream entry: очистка outbox не ломает повтор операции.

## Подготовка batch

Rust `encode_presence_items` передаёт `action`, `clientId`, `clientSegment`,
`allowed`, `hasData`, `memberJson`. Payload сериализуется один раз для каждого
элемента; отдельная копия data во входе не передаётся. `hasData` различает
отсутствие данных и явно переданный null без разбора memberJson в Lua.

После lookup вызывается `check_presence_attachment`: общий `read_attachment`
проверяет владельца и формат attachment, затем требуется режим Presence.
Connection state повторно не читается. Отсутствующий attachment даёт
`notAttached`, отсутствие режима — `presenceModeNotEnabled`.
Пустой batch основной transition должен отклонить до проверки attachment,
если канал существует, как memory store.

`prepare_presence_batch` принимает ключи channel/connection members, сегменты
канала/соединения, проверенный attachment и items. Для каждого затронутого
участника один HMGET читает payload/revision/timestamp, SISMEMBER проверяет
обратную ссылку. Повторные элементы используют подготовленное состояние.
Отказ любого элемента отбрасывает весь план без изменения Redis.

- Enter существующего участника становится Update.
- Update/Leave отсутствующего участника дают `invalidMemberState`.
- Отсутствующий client ID даёт `unidentifiedConnection`, запрещённый —
  `clientIdNotAllowed` с исходным client ID.
- Leave без data сохраняет последнее состояние, включая предыдущее изменение
  того же участника в этом batch.

План содержит `members`, `changes`, `member_delta`. `members[*].after` — индекс
items с единицы либо false для удаления. `changes[*]` содержит action и previous:
исходный payload либо индекс предыдущего элемента для Leave без data, иначе false.
Порядок changes совпадает с items. Ссылки действуют только при подготовке;
в сохранённых результатах их заменяет полный payload.

## Counters и записи результата

Основной скрипт читает восемь полей через `read_channel_state` и проверяет
переполнение Presence revision до подготовки batch, как memory store.
`prepare_presence_counters(before, plan.member_delta)` использует уже прочитанный state:

- Presence revision увеличивается один раз на успешный непустой batch.
- `presence_members` меняется на итоговый member_delta, остальные counters прежние.
- Occupancy version растёт только при ненулевом member_delta. Единственная
  изменённая категория — `presenceMembers`; zero boundary отмечается, если
  начальное или итоговое число участников равно нулю.
- Отрицательный counter даёт `corrupt_state`, превышение `2^53-1` —
  `numeric_overflow`, до любых записей.

`prepare_presence_result(plan, items, counters, channel_json, node_instance_json,
event_id, now_ms)` возвращает members для записи/удаления, общие presence_revision
и updated_at, outbox и outcome. Redis I/O и кодирования JSON здесь нет.

Outbox `presence.v2` содержит:

- Общие поля события: `event_name`, `schema_version`, `event_id`, `occurred_at_ms`,
  `channel_json`, `node_instance_json`, `presence_revision`, `occupancy_version`.
- Шесть counters; `changed.<category>` и `boundary.<category>` со значением `1`.
  Отсутствие флага означает false; category — camelCase wire name.
- `change_count` и поля `change.I.action`, `change.I.member`, начиная с I = 1.
  Member — исходный payload текущего элемента, включая исходный message ID.
- Для Leave без data — `change.I.previous`, полный последний payload участника.
  Если он создан этим же batch, используется готовый memberJson предыдущего item.

`response::decode_outbox` собирает событие из полей без чтения Redis. Для Leave
с previous берётся его data, но message ID — из текущего member. Все изменения
получают общее время Redis. При пустых changed-флагах occupancy равен None.
Успешный batch всегда создаёт событие, в том числе при нулевом member_delta.
`decode_presence` различает свежий результат, replay и ошибку хранилища.

## Commit и оставшаяся работа

После всех проверок основной скрипт записывает outbox, конечные members и их
обратные ссылки, изменённые counters/versions, результат операции и окно ledger.
Доменный отказ меняет только connection state/ledger/index поколения;
ошибка хранилища не сохраняется. Replay не выполняет записей.

XADD с переменным числом полей выполняется первой записью: ошибка Lua unpack
не оставляет изменённый state или ledger. Поля HASH результата и вытеснение
записываются небольшими командами, без unpack всего ledger. Lua не откатывает
выполненные команды при runtime error; исчерпание ресурсов Redis требует
восстановления, а ошибка транспорта не доказывает отсутствие commit.

[Detach](redis-detach.md) и [disconnect](redis-disconnect.md) реализованы; далее — cleanup поколений.
Подключение trait/runtime и outbox publisher остаётся отдельным этапом.
