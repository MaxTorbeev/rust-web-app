# Redis: detach

`RedisChannelStore::detach` вызывает `scripts::DETACH` одним Lua-запросом.
Результат разбирает общий `response::decode_transition`. Метод подключён
через `AttachmentStore` к runtime.
Двухнодовый WebSocket-тест проверяет DETACHED и доставку Leave другой ноде.

## Входы

KEYS совпадают по порядку с attach:

1. Node lease.
2. Generation deadlines.
3. Channel state.
4. Channel attachments.
5. Channel members.
6. Connection state.
7. Connection channels.
8. Connection members.
9. Generation connections.
10. Outbox.

ARGV:

1. Полное значение lease token.
2. Generation G.
3. Сегмент приложения A.
4. Сегмент канала C.
5. Сегмент соединения K.
6. Исходный JSON ChannelKey.
7. Исходный connection ID.
8. Исходный JSON NodeInstance команды.
9. Event ID команды.

Rust проверяет приложение и поколение actor через общий validate_actor.
JSON и ключи подготавливает Rust; Lua JSON не кодирует. request_time не
используется: canonical timestamp назначается по Redis TIME при проверке lease.

## Подготовка и commit

До первой записи скрипт проверяет lease/deadline, статус и поколение соединения,
владение attachment и его связь с connection channels. Generation index содержит
только открытые соединения; для закрытого state запись в SET отсутствует.
Закрытое соединение без attachment допускает повторный detach; attachment
закрытого соединения считается повреждением состояния.

При отсутствии attachment возвращается `{"unchanged", occupancy_version}`:
текущая версия канала либо 0 для отсутствующего канала. State, ledger, индексы
и outbox не меняются; полный список members в этом случае не читается.

При наличии attachment:

- Общий `prepare_detach` читает членов канала и metadata через read_presence_members одним HGETALL.
- `prepare_presence_removal` проверяет и готовит удаление всех участников
  данного соединения и их обратных ссылок, независимо от текущих effective modes.
- `prepare_attach_counters` получает старый attachment и нулевые новые counters.
  Occupancy version увеличивается один раз. Presence revision увеличивается
  только при наличии удаляемых участников. Проверяются underflow/overflow и
  соответствие общего числа members сохранённому counter.
- `prepare_removal_outbox`, общий с attach, готовит полную запись события.
- После подготовки выполняются XADD, удаление attachment и ссылки C,
  удаление K.U/K.U:revision/K.U:updated_at и ссылок C.U, запись counters/versions.

XADD выполняется первым, чтобы ошибка unpack большого списка полей возникла
до удаления state. Ошибки Redis не откатывают уже выполненные команды;
ошибка транспорта не доказывает отсутствие commit.

Состояние соединения, generation connections и Presence ledger сохраняются:
закрывает ledger и назначает retention [disconnect](redis-disconnect.md). Channel state даже
при нулевых counters сохраняется вместе с версиями.

## Событие и повтор

Outbox использует `format = detach.v1`, `node_instance_json` из команды и общие
поля события: event/schema, event ID, Redis timestamp, канал, versions/counters,
флаги Occupancy и `removed_count`/`removed.I` с исходными payload участников.
`prepare_removal_outbox` и `decode_outbox` общие с attach; формат attach.v2 сохранён.

Rust сортирует удалённых участников по client ID и формирует Leave с ID
`server:<event_id>:<index>` (индекс с нуля). Data берётся из сохранённого payload,
время каждого Leave совпадает со временем события. Если участников не было,
создаётся только Occupancy change, presence_revision равен None.

Успех возвращает `{"changed", outbox_fields}` либо `{"unchanged", version}`.
Отказ — `{0, code, message}`, разбираемый как ошибка хранилища.

Повтор после удаления, без промежуточного attach, возвращает Unchanged и
не создаёт новое событие. Результат detach не записывается в Presence ledger;
при потере ответа исходное событие остаётся в outbox. Если между вызовами
соединение присоединилось вновь, следующий detach удалит новый attachment,
как в memory store: отдельного номера операции/attachment в DetachCommand нет.

## Оставшаяся работа

[Disconnect](redis-disconnect.md) реализован; далее — cleanup поколений,
подключение store к traits/runtime и outbox publisher.

Текущий detach переиспользует проверку прямых и обратных индексов и читает весь
channel members при существующем attachment. Сокращение объёма чтений относится
к отдельному [плану оптимизации](redis-presence-optimization.md).
