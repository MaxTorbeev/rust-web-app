# Подзадача: реализовать и интегрировать RedisChannelStore

Статус: в работе; этап 1 выполнен, runtime остаётся на memory store.

Родительский дизайн: [Кластерный Presence и Ably-compatible Occupancy](presence-occupancy.md).

## Цель

Реализовать Redis-хранилище состояния каналов и подключить его к существующим
сервисам Presence и attachments. Изменения состояния должны сохраняться вместе
с durable outbox, доставляться через JetStream и проецироваться в локальные
соединения каждой ноды.

Реализация RedisStore и включение кластерного режима — разные этапы. Переключение
runtime допустимо только после готовности доставки, lifecycle-задач и проверки
сбоев на двух нодах.

## Текущее состояние

- `crates/realtime/src/channel/store/redis/` содержит структуру адаптера,
  построитель `RedisKeys` и его тесты. Store и Lua transitions ещё не реализованы.
- Есть контракты `AttachmentStore`, `PresenceStore` и `OccupancyShardStore`.
- `MemoryChannelStore` реализует attachments и Presence; есть общий набор
  store contract-тестов.
- `RealtimeApplication::new` собирает memory-режим; `with_services` позволяет
  подключить внешне собранные store и delivery.
- `NodeInstance` содержит node identity и boot generation.
- `redis-lease` предоставляет операции lease, `LeaseToken` и Lua-фрагмент
  проверки владения. Атомарное обновление generation deadline вместе с lease
  ещё требуется реализовать.
- Outbox publisher, кластерный Presence projector и reaper ещё не реализованы.
- Occupancy delivery и aggregated attachments остаются незавершёнными.

## Границы и обязательные условия

- V1 использует один логический Redis primary. Redis Cluster sharding в эту
  подзадачу не входит.
- Существующие доменные контракты и сервисы сохраняются; Redis-специфичные
  ключи, сериализация и Lua находятся в адаптере.
- Redis — authoritative state. При его недоступности memory fallback запрещён.
- Проверка lease и изменение защищаемого состояния выполняются одним Lua-вызовом.
- State, соответствующие версии, canonical event и outbox фиксируются одним
  transition. Отдельный `EventBus::publish` после записи state запрещён.
- В Redis-режиме единственный источник доставки committed events — outbox.
- Memory-режим продолжает работать и проходить свои contract-тесты.
- Guest auth, PHP SDK adapter и другие расширения внешнего API не входят в
  эту подзадачу.

## План работ

### 1. Схема ключей и структура адаптера

Выполнено. Схема, типы данных, сроки хранения и правила миграции описаны в
[Redis Presence: схема ключей v1](redis-store-schema.md). Код:
`crates/realtime/src/channel/store/redis/keys.rs`.

Преобразовать `channel/store/redis.rs` в модуль `channel/store/redis/`.
Выделить реализацию store, построение ключей, протокол данных и Lua-скрипты.

Зафиксировать schema в namespace `APP.APP_ENV.presence.v1`:

- channel state: attachments, members, Presence revision и Occupancy version;
- materialized Occupancy counters и aggregated shards;
- reverse indexes connection → channels и node generation → connections;
- operation ledger и retention закрытых соединений;
- node leases, generation registry и deadline index;
- Redis Stream outbox, dirty-channel index и publish deadlines.

Application/channel identifiers кодировать без неоднозначных разделителей.
Исключить пересечение lease-ключей со служебными fence-ключами. Описать сроки
жизни записей, условия удаления и правила изменения версии schema.

Результат: согласованная схема и `keys.rs` с проверками разделения namespace,
application и channel.

### 2. Node lease и generation registry

Определить способ передачи действующего `LeaseToken` в RedisStore и его
соответствие `NodeInstance`. Redis-специфичный token не добавлять в клиентский
протокол.

Реализовать атомарные claim/renew, которые используют Redis `TIME`, меняют lease
и обновляют deadline конкретной generation в ZSET. Последовательность
`RedisLease::renew` → отдельный `ZADD` не удовлетворяет этому контракту:
потребуется согласовать композицию Lua с `redis-lease`.

Клиентские mutation-скрипты проверяют актуальный token и generation до записей.
Определить поведение при истечении lease и неопределённом результате renewal.
Проверить точность fence и совместимость формата token на границе Rust/Lua.

### 3. Точные attachments и Presence с durable outbox

Реализовать операции в следующем порядке:

1. `attach_and_snapshot`.
2. `snapshot`.
3. `apply_presence`.
4. `detach`.
5. `disconnect`.

Первый изменяющий transition уже должен сохранять canonical event в outbox.
Событие содержит исходный `event_id`, имя, schema version и полный payload;
publisher не должен восстанавливать payload из изменяемого channel state.

Для Presence batch обеспечить проверку всего batch до изменения доменного
состояния, operation dedup по идентичности команды и проверку fingerprint.
Повтор возвращает исходный outcome/event ID. No-op и rejected outcome не
создают новую Presence revision или событие изменения состояния.

Snapshot возвращает согласованные members и версии. Detach/disconnect
обновляют reverse indexes и создают необходимые server-generated Leave events.
Disconnect сохраняет предусмотренную контрактом целостность операции по каналам.

Ожидаемые protocol rejections возвращаются как доменный outcome;
инфраструктурные и serialization failures — как store error.

Отдельно разобрать возможные ошибки после первой записи Lua. Атомарное
исполнение не обеспечивает rollback при runtime error: проверки выполняются
до записей, а оставшиеся сценарии отказов требуют явного безопасного поведения
и тестов. Пропуск номера fence допустим; потеря связи state/outbox — нет.

### 4. Проверки store-контракта

Добавить Redis-фикстуру в
`crates/realtime/tests/store_contract/implementations.rs`. Подключать общие
сценарии по мере реализации операций; изолировать ключи каждого запуска.

Дополнить набор Redis-специфичными проверками:

- потеря ответа после commit и retry с прежним outcome/event ID;
- отсутствие второй outbox entry при повторе Presence mutation;
- конфликт fingerprint для той же идентичности операции;
- конкурентные mutations и согласованность revisions/snapshot;
- истёкший token и token предыдущего периода владения;
- повреждённое состояние и ошибки Lua без незаметного частичного commit;
- согласованность state, ledger, indexes и outbox после успешного transition.

### 5. Outbox publisher и кластерный projector

Реализовать publisher с lease, чтением durable events и повторной отправкой
с тем же `event_id` при неопределённом результате публикации. Lease не отменяет
уже начавшийся сетевой publish, поэтому повторы должны безопасно обрабатываться
потребителем.

Добавить Redis-вариант `ChannelCommitDelivery`, который не публикует событие
повторно. Consumer выполняет dedup, соблюдает порядок проекции и доставляет
изменения локальным соединениям через `ChannelRouter`.

Реализовать pending attach barrier для Presence/Occupancy versions, согласование
snapshot с deltas и resync при пропуске revision. Ошибка общей проекции не должна
подтверждать успешную обработку события транспортом.

### 6. Reaper и Occupancy

Reaper находит истёкшие generations по deadline index и очищает их ограниченными
batch. Каждый batch проверяет token reaper-а, актуальность deadline и точное
владение удаляемых записей. Новая generation не должна удаляться cleanup старой.
Изменения состояния и события cleanup также фиксируются через outbox.

Реализовать `OccupancyShardStore::flush`: принимать только более новую shard
version, заменять абсолютные counters и применять разницу к общим метрикам.
Добавить versioned dirty-channel publication, initial Occupancy snapshot и
доставку последующих изменений. Повтор flush не удваивает counters; publication
старой версии не очищает более новый dirty marker.

### 7. Сборка runtime и включение Redis-режима

Подключить согласованную пару store/delivery через
`RealtimeApplication::with_services`. Добавить выбор драйвера и валидацию
совместимости с EventBus: кластерный Presence требует общей durable доставки.

Запускать renewer, publisher, consumer и reaper под supervision. Готовность
трафика зависит от работоспособности обязательных компонентов. При потере
node lease прекращать mutations, снимать readiness и завершать соответствующий
runtime с закрытием соединений.

До включения кластерного режима провести live-проверки с двумя нодами,
реальными Redis и JetStream: cross-node delivery, retry, restart, outage,
lease expiry, reaper cleanup, snapshot/delta race и resync.

## Критерии завершения

- Redis-реализация проходит общий контракт attachments/Presence и проверки
  Redis-специфичных отказов; memory-проверки продолжают проходить.
- Успешный ACK Presence mutation означает durable commit state и outbox,
  а не завершение доставки всем подписчикам.
- Повтор операции не создаёт второе доменное изменение или второй event ID.
- Старый период владения не может менять защищённое состояние.
- Node lease и generation deadline обновляются атомарно.
- События восстанавливаются для отправки из outbox без чтения mutable state.
- Consumer/projector безопасно обрабатывает повторную доставку и gaps.
- Reaper очищает умершую generation, сохраняя состояние новой generation.
- Aggregated Occupancy не удваивается от повторных flush и не теряет новые
  изменения при завершении publication старой версии.
- Redis outage не приводит к локальному fallback или успешным ложным ACK.
- Live two-node failure-сценарии пройдены; команды и результаты проверки
  сохранены перед включением режима.

## Следующий шаг

Реализовать node lease + generation deadline на определённой схеме ключей.
Затем реализовать первый `attach_and_snapshot` с fencing и outbox в одном
transition.
