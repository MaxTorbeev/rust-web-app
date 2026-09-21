# Подзадача: реализовать и интегрировать RedisChannelStore

Статус: точный Presence интегрирован с Redis, JetStream и WebSocket runtime.
По умолчанию остаётся `memory`; Redis включается явно. Aggregated Occupancy и
доставка Occupancy — отдельный незавершённый этап.

Родительский дизайн: [Кластерный Presence и Ably-compatible Occupancy](presence-occupancy.md).

План оптимизации: [перенос нагрузки Redis Presence на realtime-ноды](redis-presence-optimization.md).
Приоритет — отделение metadata от payload, затем локальные проекции для snapshot.
Подготовка batch в Rust с условным commit рассматривается отдельно; изменения
схемы и контракта snapshot пока не реализованы.

## Цель

Реализовать Redis-хранилище состояния каналов и подключить его к существующим
сервисам Presence и attachments. Изменения состояния должны сохраняться вместе
с durable outbox, доставляться через JetStream и проецироваться в локальные
соединения каждой ноды.

Реализация RedisStore и включение кластерного режима — разные этапы. Переключение
runtime допустимо только после готовности доставки, lifecycle-задач и проверки
сбоев на двух нодах.

## Текущее состояние

- `RedisChannelStore` реализует `AttachmentStore` и `PresenceStore`: attach,
  snapshot, Presence batch, detach и disconnect. Lua атомарно меняет state,
  ledger, indexes и outbox; сериализация публичного JSON выполняется в Rust.
- `NodeLease` связывает token с `NodeInstance`; claim/renew одним Lua-вызовом
  обновляют lease и generation deadline по Redis TIME.
- `RedisPresenceRuntime` запускает renewal, outbox publisher и reaper.
  Publisher сохраняет event ID при повторе и удаляет outbox entry только после
  подтверждения JetStream и повторной проверки fencing.
- Consumer доставляет Presence через локальный `ChannelRouter`, учитывая
  effective modes и revision каждого подписчика. Повторы пропускаются,
  пропуск revision восстанавливается authoritative snapshot через SYNC.
- Pending ATTACH регистрируется до store transition. Он хранит максимальную
  увиденную revision; устаревший snapshot перечитывается. ATTACHED/SYNC ставятся
  в очередь под lock перед активацией deltas. `channelSerial` содержит пару
  Presence/Occupancy versions; полноценный resume этим не реализуется.
- Reaper очищает соединения истёкших generations порциями и финализирует registry.
  Новое поколение той же ноды и fence counters не удаляются.
- Ошибка обязательной задачи завершает runtime, снимает readiness и закрывает
  локальные соединения. Неопределённый renewal требует перезапуска процесса.
  Ошибка Presence projection возвращается consumer-у как retryable и сразу
  запрещает новые mutations; node renewer завершает runtime на следующем проходе.
- Occupancy delivery и aggregated attachments остаются незавершёнными.

## Включение

```dotenv
PRESENCE_STORE_DRIVER=redis
EVENT_BUS_DRIVER=nats
```

Также нужны доступные Redis и JetStream, общие `APP`/`APP_ENV` и настройки
EventBus для нод кластера, уникальный `APP_NODE_ID` для каждого одновременно
работающего процесса. Boot generation создаётся при каждом запуске.
Redis с in-memory EventBus отклоняется при старте; fallback отсутствует.
`PRESENCE_STORE_DRIVER=memory` сохраняет автономный режим.

Node/publisher/cleanup lease: 15 секунд; node и publisher renewal: 5 секунд.
Publisher читает до 32 событий за проход, при пустом outbox ждёт 250 мс.
Reaper выбирает до 32 generations и до 32 соединений каждой за проход,
между проходами ждёт секунду. Это размеры фоновых порций, не лимиты Presence.
После остановки ноды её состояние удаляется reaper-ом после истечения lease.
Redis persistence/HA и настройки retention JetStream задаются эксплуатацией;
успех тестов на временных сервисах не проверяет их production-конфигурацию.

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

Выполнено.

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

Выполнено для `AttachmentTracking::Individual`.

Реализовать операции в следующем порядке:

1. `attach_and_snapshot`.
2. `snapshot`.
3. `apply_presence`.
4. `detach`.
5. `disconnect`.

Первый изменяющий transition уже должен сохранять полные неизменяемые данные
canonical event в outbox: исходный `event_id`, имя, schema version, timestamp
и все данные payload. Публичный JSON собирает Rust только из этой записи;
publisher не должен восстанавливать payload из изменяемого channel state.
Внутренний формат outbox версионируется отдельно от публичного события;
для attach он описан в [контракте attach_and_snapshot](redis-attach-and-snapshot.md).

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

Redis-проверки находятся в `redis/node/tests/` и `redis/integration_tests.rs`.
Общий шаблон ниже пока запускается только для memory: он использует управляемый
request clock и произвольные поколения actor, тогда как Redis использует TIME
и lease текущего запуска. Его перенос на общую clock/lease-фикстуру остаётся
отдельной задачей; Redis проверяется через публичные store traits и live runtime.

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

Выполнено для Presence. Occupancy projection остаётся отдельным этапом.

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

Реализованы [поиск кандидатов и очистка поколения](redis-reaper.md):
expired_generations выбирает ограниченное число поколений по Redis TIME,
reap_connection очищает одно соединение под node lease reaper-а и cleanup lease
цели, переиспользуя закрытие disconnect. `reap_generation_batch` выбирает порцию
connections и финализирует пустое поколение; runtime выполняет фоновый обход.

Реализовать `OccupancyShardStore::flush`: принимать только более новую shard
version, заменять абсолютные counters и применять разницу к общим метрикам.
Добавить versioned dirty-channel publication, initial Occupancy snapshot и
доставку последующих изменений. Повтор flush не удваивает counters; publication
старой версии не очищает более новый dirty marker.

### 7. Сборка runtime и включение Redis-режима

Выполнено для точного Presence; выбор драйвера описан выше.

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

## Проверка интеграции

Команды для выделенных временных Redis и JetStream:

```sh
cargo test --workspace
REALTIME_REDIS_TEST_PORT=16389 cargo test -p realtime --lib -- --ignored
cargo build -p mxt-realtime
REALTIME_REDIS_TEST_PORT=16389 REALTIME_NATS_TEST_URL=nats://127.0.0.1:14222 \
  cargo test -p realtime --test redis_cluster -- --ignored --nocapture
```

`redis_cluster` запускает два настоящих процесса приложения с изолированными
namespace/stream и WebSocket-клиентами. Проверяет cross-node Enter, повтор
команды без второго события, initial snapshot, аварийную остановку ноды,
lease expiry и server-generated Leave, новый boot, disconnect и отказ Redis
без успешного ACK. Последний сценарий использует `CLIENT PAUSE ALL`, поэтому
Redis для этого теста обязан быть выделенным и временным. Логи процессов
сохраняются в напечатанном тестом временном каталоге.

Дополнительно Redis unit/integration tests проверяют старые tokens, неизвестный
результат публикации с повтором исходного event ID, fencing publisher-а,
сохранение нового поколения при cleanup, ATTACH race, revision gap и dedup.

Проверено на временных Redis 7.2.7 и NATS JetStream 2.11.17:
`cargo test --workspace` прошёл; отдельно прошли все 20 Redis-тестов realtime
и двухнодовый сценарий (включая DETACH и Redis outage). Workspace-прогон
содержит 51 memory store contract-тест; это не 51 проверка Redis-реализации.
Проверены также остановка mutations после ошибки projector и отмены runtime.

## Оставшиеся отдельные этапы

- Aggregated attachments, `OccupancyShardStore`, dirty publication и Occupancy delivery.
- Общая Redis/memory фикстура для всех store contract-тестов.
- Нагрузочные измерения и оптимизации из отдельного плана; бизнес-лимиты пока не вводятся.
