# Redis: контракт attach_and_snapshot

Статус: Lua transition реализован в `scripts/attach_and_snapshot.lua`.
Фрагменты собираются при компиляции в `scripts::ATTACH_AND_SNAPSHOT`.
Rust-метод `RedisChannelStore::attach_and_snapshot` вызывает скрипт и разбирает
ответ через `response::decode_attach`. Подключён через `AttachmentStore` к
runtime; проверены live Redis и ATTACH barrier с snapshot/revision race.

Связанные документы: [схема ключей](redis-store-schema.md),
[план интеграции, этап 3](redis-store-integration.md#3-точные-attachments-и-presence-с-durable-outbox).

Операция принимает `AttachCommand` с `AttachmentTracking::Individual` и
возвращает `ChannelAttachOutcome`. Она сохраняет attachment и читает snapshot
после изменения одним Lua-вызовом. Повтор обновляет параметры attachment и
возвращает свежий snapshot; одинаковый повтор не увеличивает counters и не
создаёт событие. Aggregated attachments реализуются отдельно.

## Входы

Обозначения `A`, `C`, `K`, `U`, `G` соответствуют схеме ключей. Rust строит
все ключи через `RedisKeys` одного store, а сегменты — через `protocol::segment`.
Клиент не передаёт готовые Redis-ключи, generation или token.

| KEYS | Построитель RedisKeys | Тип |
|---|---|---|
| 1 | `node_lease(node_id)` | STRING |
| 2 | `generation_deadlines()` | ZSET |
| 3 | `channel_state(channel)` | HASH |
| 4 | `channel_attachments(channel)` | HASH |
| 5 | `channel_members(channel)` | HASH |
| 6 | `connection_state(application, connection)` | HASH |
| 7 | `connection_channels(application, connection)` | SET |
| 8 | `connection_members(application, connection)` | SET |
| 9 | `generation_connections(instance)` | SET |
| 10 | `outbox()` | STREAM |

| ARGV | Значение |
|---|---|
| 1 | `redis_lease::lease_value(node_lease.token())`: полный token |
| 2 | `protocol::generation(node_lease.instance())`: `G` |
| 3 | `A`: закодированный application ID |
| 4 | `C`: закодированное имя канала |
| 5 | `K`: закодированный connection ID |
| 6 | JSON `command.channel` в существующем serde-формате |
| 7 | JSON `command.to_attachment()` в существующем serde-формате |
| 8 | `command.event_id`: каноническая строка UUID |

Rust вызывает `validate_attach` перед сериализацией. JSON, сегменты и ключи
строятся из одной проверенной команды; instance команды соответствует
`node_lease.instance()` по node ID и boot generation. `started_at` — метаданные.
`request_time` не передаётся как время commit: canonical timestamp вычисляет
Lua через Redis `TIME`.

Скрипт — внутренний API Rust-адаптера: UUID, NodeId, accounting и effective
modes проверяются типами Rust и `validate_attach`. Lua не повторяет эти
проверки новой команды и не разбирает JSON канала. Это не отменяет проверки
уже сохранённого в Redis состояния. Прямой вызов с непроверенной командой
не является поддерживаемым способом использования скрипта.

## Проверки и подготовка до первой записи

1. Проверить число KEYS/ARGV, декодировать attachment и проверить соответствие
   его поколения `G`. Формат входной команды обеспечивает Rust-адаптер.
2. Для connection/channel state определить наличие ключа через TYPE и передать
   признаки в helpers. Типы читаемых ключей проверяют сами GET, HMGET, HGET,
   HGETALL, SISMEMBER, SCARD и SMEMBERS до первой записи. Отсутствующие ключи
   данных допустимы при создании нового состояния; отсутствие lease/deadline
   означает потерю владения. Повреждённое существующее состояние не считать пустым.
   Если требуется XADD, отдельно проверить TYPE outbox: `none` или `stream`.
3. Проверить token через встроенный `LUA_HOLDS_LEASE`, соответствие его owner
   поколению `G` и наличие deadline `G`, строго большего текущего Redis TIME.
   Сохранить это же время для canonical event. Fence сравнивать строкой.
4. Прочитать connection state: закрытое соединение и другая generation дают
   конфликт. Проверить владельца и individual accounting существующего
   attachment. Не разрешать перехват старого соединения новым boot.
5. Прочитать channel state, attachment и members вместе с обратными индексами.
   Проверить поля, необходимые для mutation, и принадлежность удаляемых members
   соединению и generation. Несогласованные индексы отклонить до изменений.
   Наличие attachment должно совпадать с наличием `C` в connection channels,
   а наличие открытого connection state — с наличием `A.K` в generation connections.
   Connection indexes без connection state и attachments без channel state
   считаются повреждённым состоянием.
   Полная десериализация JSON участников выполняется в Rust (см. ниже).
6. Определить, какие счётчики Occupancy увеличивал старый attachment и какие
   должен увеличивать новый, и вычислить изменение каждого счётчика. Если
   повтор убирает режим `Presence`, подготовить удаление всех members этого
   соединения в данном канале, включая записи `C.U` в connection index.
7. Проверить переполнение/уменьшение ниже нуля всех затронутых counters и
   versions в диапазоне `0..9 007 199 254 740 991` (`2^53 - 1`).
   Увеличить запланированную `presence_revision` один раз, только
   если удаляются members; `occupancy_version` — один раз при изменении metrics.
8. Подготовить массивы исходных данных snapshot/transition и все строковые поля
   outbox до первой записи. План удаления уже построен из того же неизменённого
   массива members; повторно проверять его не требуется. Проверить соответствие
   итогового числа members счётчику. Ошибки Lua-проверок и кодирования
   должны быть обнаружены до записей: runtime error Lua не откатывает их.

Outbox принадлежит адаптеру и пополняется только через `XADD *`. Проверка
максимального Stream ID через XINFO не выполняется: ручное изменение ID
stream не входит в поддерживаемый контракт. Ошибка Redis во время commit
по-прежнему не означает откат уже выполненных записей.

Для нового пустого канала обе versions и шесть counters равны нулю. При
обновлении счётчиков канала вычитаем значения, учтённые для старого attachment,
и прибавляем значения для нового. Остальные подключения, включая будущие
aggregated shards, продолжают учитываться без изменений.

Для старого и нового attachment один раз вычисляем, какие счётчики Occupancy
он увеличивает на 1. Полученные значения используем для определения потери
режима Presence и обновления счётчиков канала, без повторного обхода effective
modes. Например, attachment с режимами `subscribe` и `presence` увеличивает
`connections`, `subscribers` и `presence_connections` на 1; для остальных
двух счётчиков возвращается 0. Если оставить только `subscribe`, при обновлении
канала `presence_connections` уменьшится на 1, а остальные четыре счётчика
не изменятся. Участники `presence_members` считаются отдельно: одно подключение
может представлять нескольких участников Presence.

Порядок вызовов в transition:

```lua
local old = attachment_occupancy(previous)
local new = attachment_occupancy(attachment)
local removed = {}
if old.presence_connections == 1 and new.presence_connections == 0 then
  removed, rejection = prepare_presence_removal(members, KEYS[8], ARGV[4], ARGV[5], attachment)
  if rejection then return rejection end
end
local counters, rejection = prepare_attach_counters(KEYS[3], old, new, removed, state_exists)
if rejection then return rejection end
```

Отсутствующему старому attachment соответствует `old.connections == 0`;
существующему individual attachment — `1`. Расчёт counters не изменяет
`old` и `new`; условие потери Presence проверяется вызывающим transition.

Members канала читаются одним `HGETALL` внутри transition, после проверки lease.
Полученный массив field/{payload, revision, timestamp} передаётся в `prepare_presence_removal`, которая
его не изменяет. Snapshot строится из того же массива с исключением полей из
плана удаления; повторного чтения HASH нет. Проверка прямого и обратного
индексов сохраняется. Передавать members, прочитанные отдельным Redis-запросом
до вызова transition, нельзя: это нарушило бы атомарность snapshot.

## Сборка Lua

Основной файл использует локальные функции из предшествующих фрагментов.
Для одного Lua-вызова соединить через переводы строк в следующем порядке:

1. `LUA_CHECK_NODE_LEASE` (уже включает `LUA_HOLDS_LEASE`).
2. `LUA_CHECK_ATTACH_STATE` (включает общий `read_attachment`).
3. `attachment_occupancy.lua`.
4. `read_presence_members.lua`, затем `prepare_presence_removal.lua` (также определяет `member_segment`).
5. `read_channel_state.lua`.
6. `prepare_attach_counters.lua`.
7. `append_occupancy_fields.lua`, `prepare_removal_outbox.lua` и `prepare_attach_result.lua`.
8. `attach_and_snapshot.lua`.

Сборка определена в `scripts/mod.rs` через `concatcp!`, как для node lease.
Эти фрагменты нельзя выполнять отдельными Redis-запросами: проверка lease,
чтение, подготовка и commit должны оставаться одним вызовом.

## Commit

- Создать connection state с `generation = G`, `status = open`, если его нет.
  При повторе сохранить остальные поля, включая `highest_serial`.
- Сохранить JSON attachment в field `K` channel attachments.
- Добавить `C` в connection channels и `A.K` в generation connections.
- Удалить подготовленные members `K.U` и соответствующие ссылки `C.U`.
  Ссылки на другие каналы оставить. Attach не закрывает соединение и не
  устанавливает TTL на его state или ledger.
- Записать вычисленные counters и versions в channel state.
- Если метрики изменились, добавить полные неизменяемые данные события в outbox.
  Если изменений метрик нет, вернуть `Unchanged` без outbox entry.
- Вернуть snapshot после этой операции и тот же transition, который был
  подготовлен для commit. Все шаги выполняются внутри одного Lua-вызова.

Attach не создаёт записи Presence operation ledger: у `AttachCommand` нет
`msg_serial`. Идемпотентность здесь определяется сохранённым attachment.
После потери ответа повтор возвращает свежий snapshot и может быть `Unchanged`;
ранее созданное событие остаётся в outbox со своим исходным event ID.

## Событие и точность данных

Lua не собирает публичный JSON события. Outbox хранит полные неизменяемые
данные для его сборки в Rust. Внутренний формат attach имеет следующие поля:

| Поле | Значение |
|---|---|
| `format` | `attach.v2`, версия внутреннего формата записи |
| `event_name` | `realtime.presence_channel_changed` |
| `schema_version` | `1`, версия публичного события, независимая от `format` |
| `event_id` | Исходный UUID команды |
| `occurred_at_ms` | Время Redis TIME, целая десятичная строка |
| `channel_json` | Исходный JSON `ChannelKey` |
| `attachment_json` | Исходный JSON нового attachment; его `nodeInstance` задаёт origin |
| `presence_revision` | Новая ревизия при удалении members; пустая строка для Occupancy-only |
| `occupancy_version` | Версия после изменения, десятичная строка в диапазоне `0..2^53-1` |
| `connections`, `publishers`, `subscribers`, `presence_connections`, `presence_subscribers`, `presence_members` | Итоговые метрики, десятичные строки в диапазоне `0..2^53-1` |
| `changed.<category>`, `boundary.<category>` | Флаг `1`; отсутствие означает false. Категории — camelCase wire names |
| `removed_count` | Число удалённых участников |
| `removed.I` | Исходный JSON MemberPayload удалённого участника; I начинается с 1 |

Lua не кодирует JSON. Переменные списки представлены отдельными полями, payload
сериализован Rust. Для пустого списка достаточно `removed_count = 0`.

Rust декодирует запись одним общим преобразованием для результата attach и
outbox publisher. Неизвестный `format` отклоняется. Rust извлекает origin из
attachment, разбирает метрики/версии как `u64`, сортирует удалённых участников
по исходному `client_id` и создаёт Leave с ID `server:<event_id>:<index>`
(индекс с нуля). Время всех Leave, события и envelope — `occurred_at_ms`.
Пустая `presence_revision` становится `None`; метрики и категории образуют
`OccupancyChange`. Event ID при повторной обработке не генерируется заново.

Publisher формирует `PresenceChannelChanged` только из outbox entry, без
чтения channel state или часов приложения. Падение между commit и сборкой
публичного JSON не теряет событие. Ошибка декодирования записи не разрешает
подтверждать её доставку или удалять её из outbox.

Snapshot содержит все оставшиеся members, обе текущие versions и полные metrics.
Rust десериализует участников и сортирует по исходным `(connection_id, client_id)`.
Lua возвращает тройки `{payload_json, revision, updated_at}` без сортировки и перекодирования. JSON записывается
из типизированных Rust-значений; Lua проверяет только поля, необходимые для
mutation. Полная проверка остальных полей происходит при десериализации в Rust.
При повреждении этих полей ошибка может обнаружиться уже после commit; это
не означает откат операции. Outbox остаётся доступным для диагностики/повтора.

Шесть counters канала, `presence_revision` и `occupancy_version` ограничены
диапазоном `0..9 007 199 254 740 991` (`2^53 - 1`). Rust-типы остаются `u64`.
Lua проверяет целую каноническую десятичную запись и диапазон, затем считает
обычными числовыми операциями. Проверки уменьшения ниже нуля и переполнения
выполняются до соответствующих вычитания и сложения, до любых записей Redis.
Сохранённое значение вне диапазона даёт `corrupt_state`, выход за границу
при расчёте — `numeric_overflow`. Версия на максимуме допустима для операции,
которая её не увеличивает; переполнение не приводит к сбросу или насыщению.

Для хранения в Redis, outbox и ответа Lua числа форматируются через
`string.format("%.0f", value)`: сохраняются десятичные строки без округления
значащих цифр или экспоненты. Структура ответа и разбор через Rust `parse::<u64>()`
не меняются. Строковая арифметика по цифрам больше не используется.

Этот предел не распространяется на lease fence, msg_serial, Redis Stream ID
и произвольный member `data`. JSON участников и attachment по-прежнему передаётся
без перекодирования через cjson; Rust использует `serde_json`, сохраняя точность
целых `u64` в member `data`.

## Ответ Lua

Успех: `{1, snapshot, transition}`, вложенные RESP-массивы:

- `snapshot = {members, presence_revision, occupancy_version, metrics}`.
  `members` — массив троек `{payload_json, revision, updated_at}` оставшихся участников. Обе версии —
  десятичные строки. `metrics` — плоский массив field/value для шести метрик
  в порядке таблицы выше; значения — десятичные строки.
- `transition = {"unchanged", occupancy_version}` для no-op.
- `transition = {"changed", outbox_fields}` при изменении. `outbox_fields` —
  тот же плоский массив строк field/value, который передаётся в `XADD`.

`prepare_attach_result` возвращает `{snapshot, transition, outbox}` как Lua
структуру с именованными полями. Основной скрипт использует `outbox` для XADD
(либо `false` для no-op), а клиенту возвращает массив `{1, snapshot, transition}`.
Rust собирает `ChannelAttachOutcome`; публичные типы и их serde-форматы не меняются.

Ожидаемый отказ до записей: `{0, code, message}`.

| code | Ошибка Rust |
|---|---|
| `invalid_request` | `ChannelStateStoreError::InvalidRequest` |
| `lease_lost` | `ChannelStateStoreError::Conflict` |
| `connection_closed` | `ChannelStateStoreError::Conflict` |
| `generation_mismatch` | `ChannelStateStoreError::Conflict` |
| `corrupt_state` | `ChannelStateStoreError::Internal` |
| `numeric_overflow` | `ChannelStateStoreError::Internal` |

Redis/transport errors, ошибки десериализации и неожиданный формат ответа
становятся `Internal`. Такая ошибка не доказывает отсутствие commit; нельзя
выполнять отдельную публикацию события или переходить на memory store. `lease_lost` не разрешает
автоматически захватить новый lease и продолжить старую команду.

## Проверки реализации

- Первый attach: state, индексы, metrics, snapshot и одна outbox entry.
- Одинаковый повтор: свежий snapshot, прежние versions, нет нового события.
- Изменение modes: корректная разница counters; потеря Presence удаляет members
  только этого соединения и канала, создаёт одну revision и ordered Leave.
- Истёкший lease, старый token, другой boot и закрытое соединение: никаких записей.
- Ошибки полей, проверяемых Lua, Redis types, переполнение и несогласованные
  индексы: отказ до изменения state и outbox.
- Повреждённый JSON snapshot/outbox: Rust возвращает ошибку; commit не считается
  отменённым, outbox entry не подтверждается и не удаляется.
- Сборка события после перезапуска только из outbox: исходные ID, timestamp,
  версии, data и порядок Leave сохраняются без чтения channel state.
- Граница `2^53 - 1`: точное сохранение, увеличение до максимума, отказ при
  переполнении, отсутствие увеличения версий при одинаковом повторе.
- Сохранённые counters/versions выше максимума, дроби, экспоненты, отрицательные
  значения и отсутствующие поля: отказ до записей.
- Unicode/пустые сегменты и большие целые `u64` в member data: отсутствие
  потери точности и стабильная сортировка.
- Потеря ответа после commit: повтор не удваивает counters и outbox event.

Формат members общий с [apply_presence](redis-apply-presence.md): три поля HASH
на участника. Attach удаляет payload/revision/updated_at вместе с обратной
ссылкой, а snapshot получает все три значения одним HGETALL. XADD выполняется
первой записью после подготовки: ошибка unpack большого списка outbox-полей
возникает до изменения state.

Подготовка удаления участников, counters и outbox также используется в [detach](redis-detach.md).
