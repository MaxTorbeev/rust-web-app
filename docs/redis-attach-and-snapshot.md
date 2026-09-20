# Redis: контракт attach_and_snapshot

Статус: контракт для реализации `scripts/attach_and_snapshot.lua` и его Rust
обвязки. Сам transition пока не реализован и не подключён к runtime.

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

## Проверки и подготовка до первой записи

1. Проверить число KEYS/ARGV, формат token/generation, UUID и структуру JSON.
   Проверить individual accounting и непустые effective modes.
2. Проверить Redis types всех десяти ключей. Отсутствующие ключи данных
   допустимы при создании нового состояния; отсутствие lease/deadline означает
   потерю владения. Повреждённое существующее состояние не считать пустым.
3. Проверить token через встроенный `LUA_HOLDS_LEASE`, соответствие его owner
   поколению `G` и наличие deadline `G`, строго большего текущего Redis TIME.
   Сохранить это же время для canonical event. Fence сравнивать строкой.
4. Прочитать connection state: закрытое соединение и другая generation дают
   конфликт. Проверить владельца и individual accounting существующего
   attachment. Не разрешать перехват старого соединения новым boot.
5. Прочитать channel state, attachment и members вместе с обратными индексами.
   Проверить форматы записей и принадлежность удаляемых members соединению и
   generation. Несогласованные данные отклонить до изменений.
6. Вычислить разницу вклада старого и нового attachment в Occupancy. Если
   повтор убирает режим `Presence`, подготовить удаление всех members этого
   соединения в данном канале, включая записи `C.U` в connection index.
7. Проверить переполнение/уменьшение ниже нуля всех затронутых counters и
   versions. Увеличить запланированную `presence_revision` один раз, только
   если удаляются members; `occupancy_version` — один раз при изменении metrics.
8. Полностью сформировать будущий snapshot, transition и outbox payload,
   включая сериализацию, до первой записи. Все ожидаемые ошибки должны быть
   обнаружены на этой стадии: runtime error Lua не откатывает предыдущие записи.

Для нового пустого канала обе versions и шесть counters равны нулю. Обновление
materialized counters применяет только разницу individual-вклада, сохраняя
вклад остальных attachments и будущих aggregated shards.

## Commit

- Создать connection state с `generation = G`, `status = open`, если его нет.
  При повторе сохранить остальные поля, включая `highest_serial`.
- Сохранить JSON attachment в field `K` channel attachments.
- Добавить `C` в connection channels и `A.K` в generation connections.
- Удалить подготовленные members `K.U` и соответствующие ссылки `C.U`.
  Ссылки на другие каналы оставить. Attach не закрывает соединение и не
  устанавливает TTL на его state или ledger.
- Записать вычисленные counters и versions в channel state.
- Если метрики изменились, добавить одно полное canonical event в outbox.
  Если изменений метрик нет, вернуть `Unchanged` без outbox entry.
- Вернуть snapshot после этой операции и тот же transition, который был
  подготовлен для commit. Все шаги выполняются внутри одного Lua-вызова.

Attach не создаёт записи Presence operation ledger: у `AttachCommand` нет
`msg_serial`. Идемпотентность здесь определяется сохранённым attachment.
После потери ответа повтор возвращает свежий snapshot и может быть `Unchanged`;
ранее созданное событие остаётся в outbox со своим исходным event ID.

## Событие и точность данных

Outbox entry содержит поля `event_name` (`realtime.presence_channel_changed`),
`schema_version` (`1`), `event_id`, `occurred_at_ms` и `payload` (полный JSON
`PresenceChannelChanged`). Время в envelope и payload одинаковое, из Redis TIME.
Publisher не достраивает payload из изменяемого состояния.

Payload содержит channel, origin, текущую occupancy version, полные metrics,
changed/zero-boundary categories и member changes. Для Occupancy-only изменения
`presenceRevision = null`; при удалении members — новая Presence revision.
Server-generated Leave идут по исходному `client_id`, с message ID
`server:<event_id>:<index>` (индекс с нуля) и общим canonical timestamp.

Snapshot содержит все оставшиеся members, обе текущие versions и полные metrics.
Порядок members — по исходным `(connection_id, client_id)`, не по base64-сегментам.

Counters и versions имеют диапазон `u64`: Redis хранит их десятичными строками,
а JSON — целыми числовыми литералами без потери точности. Нельзя проводить их
через Lua `tonumber`/`cjson.encode` или полагаться на signed `HINCRBY` для всего
диапазона `u64`. Нужны точные проверки и арифметика десятичных строк.
Произвольный member `data` также нельзя декодировать и затем кодировать через
Lua cjson с потерей больших целых: JSON-фрагменты должны сохраняться без потерь.

## Ответ Lua

Успех: `{1, snapshot_json, transition_json}`. Обе строки используют текущие
serde-форматы `PresenceSnapshot` и `CommittedChannelTransition`; Rust декодирует
их в `ChannelAttachOutcome`. `Changed` содержит тот же event ID и payload, что
outbox, `Unchanged` — текущую occupancy version.

Ожидаемый отказ до записей: `{0, code, message}`.

| code | Ошибка Rust |
|---|---|
| `invalid_request` | `ChannelStateStoreError::InvalidRequest` |
| `lease_lost` | `ChannelStateStoreError::Conflict` |
| `connection_closed` | `ChannelStateStoreError::Conflict` |
| `generation_mismatch` | `ChannelStateStoreError::Conflict` |
| `corrupt_state` | `ChannelStateStoreError::Internal` |
| `numeric_overflow` | `ChannelStateStoreError::Internal` |

Redis/transport errors и неожиданный формат ответа становятся `Internal`.
Transport error не доказывает отсутствие commit; нельзя выполнять отдельную
публикацию события или переходить на memory store. `lease_lost` не разрешает
автоматически захватить новый lease и продолжить старую команду.

## Проверки реализации

- Первый attach: state, индексы, metrics, snapshot и одна outbox entry.
- Одинаковый повтор: свежий snapshot, прежние versions, нет нового события.
- Изменение modes: корректная разница counters; потеря Presence удаляет members
  только этого соединения и канала, создаёт одну revision и ordered Leave.
- Истёкший lease, старый token, другой boot и закрытое соединение: никаких записей.
- Повреждённые JSON/Redis types, переполнение и несогласованные индексы:
  ошибка до изменения state и outbox.
- Значения выше `2^53`, граница `u64`, Unicode/пустые сегменты, большие целые
  в member data: отсутствие потери точности и стабильная сортировка.
- Потеря ответа после commit: повтор не удваивает counters и outbox event.
