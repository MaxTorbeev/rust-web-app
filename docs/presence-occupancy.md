# Кластерный Presence и Ably-compatible Occupancy

## Статус и границы совместимости

Это целевой дизайн и план реализации. Redis-backed Presence, leases, outbox и
Occupancy из этого документа пока не реализованы.

Ably-compatible realtime-профиль для Pub/Sub и Presence из шести метрик:

- `connections`;
- `publishers`;
- `subscribers`;
- `presenceConnections`;
- `presenceSubscribers`;
- `presenceMembers`.

## Текущее состояние

Сейчас каждый `RealtimeApplication` создаёт собственные process-local
`ChannelHub` и `PresenceHub`.

`PresenceHub` хранит:

- `channel -> connection_id -> PresenceMessage`;
- обратный индекс `connection_id -> channels`;
- не более одного member на connection в одном channel.

Входящий `clientId` заменяется `clientId` соединения. `ATTACH` сначала
регистрирует локальный sender, затем читает локальный snapshot и отправляет
`ATTACHED` и `SYNC`. Snapshot не имеет revision, а между его чтением и локальной
рассылкой delta нет barrier.

`ENTER`, `UPDATE`, `LEAVE`, `DETACH` и disconnect меняют только process-local
state и рассылаются только через локальный `ChannelHub`. События
`WebsocketConnected` и `WebsocketDisconnected` остаются `LocalOnly`.
`ChannelMessageSubmitted` уже имеет `DeliveryClass::AllNodes`, но это не делает
Presence кластерным.

В wire types пока отсутствуют необходимые части Occupancy-контракта:

- `ProtocolMessage.params`;
- `channelSerial`;
- requested и effective channel modes;
- capability checks для attach, publish и presence;
- операция capability `channel-metadata`;
- `Message.id`, `Message.timestamp`, `Message.connectionId` и
  `Message.encoding`.

`PRESENCE_STORE_DRIVER`, `PresenceStore`, `InMemoryPresenceStore` и
`RedisPresenceStore` сейчас в коде и конфигурации отсутствуют. Ниже это целевые
компоненты, а не описание уже работающего runtime.

Текущий `POST /auth/realtime/{application_id}/token` также является временным
application API: он принимает только `clientId`, возвращает внутренний
`ApiResponse` и выдаёт wildcard `publish/subscribe/presence`. Он ещё не является
Ably SDK `authUrl` или стандартным `/keys/{keyName}/requestToken`. Целевой
совместимый контракт описан в разделе [Token authentication и Ably SDK](#token-authentication-и-ably-sdk).

## Цели

- Единый authoritative snapshot Presence для всех realtime-нод.
- Атомарные `attach`, `enter`, `update`, `leave`, `detach` и disconnect
  transitions.
- Durable связь между Redis state и кластерным событием без dual-write gap.
- Publish ACK, consumer ACK, retry и deduplication с честной
  at-least-once семантикой.
- Очистка attachments и members после аварии соединения или ноды.
- Ably-compatible realtime Occupancy в заявленных границах v1.
- Один доменный контракт для memory- и Redis-реализаций.
- Сохранение `ChannelHub` как локальной точки доставки WebSocket-кадров.

## Высокая нагрузка: anonymous guest audience

Гость является realtime attachment, но не Presence member. Он может получать
обычные сообщения, Presence `SYNC` и Presence deltas, однако не может выполнять
`ENTER`, `UPDATE`, `LEAVE` или publish. Поэтому guest-соединения не попадают в
список Presence, но входят в стандартную Occupancy-метрику `connections`.

Для identified attachments и Presence members сохраняется точное хранение по
connection. Для гостевых read-only attachments используется локальный
абсолютный shard на каждой ноде:

```text
(application_id, channel, node_id, boot_generation)
    -> version, connections, subscribers, presence_subscribers, lease_deadline
```

`ATTACH`/disconnect гостя изменяет shard в памяти ноды. Нода сбрасывает
последнее абсолютное значение в Redis не реже одного раза в секунду с jitter
`±10%`. Redis Lua принимает только более новую `version`, заменяет значение
shard-а и пересчитывает общий gauge. Отдельная запись attachment и Presence
event для каждого гостя не создаётся.

Publisher помечает изменившийся channel dirty и отправляет один полный
Occupancy snapshot не чаще одного раза в секунду. Переход общего значения
`0 ↔ >0` отправляется немедленно. Повторная публикация использует тот же
`occupancy_version` и payload.

Нагрузка ограничивается числом dirty `(node, channel)`, а не числом быстрых
connect/disconnect:

```text
Redis shard updates  ≈ active_nodes × dirty_channels / 1s
Occupancy events      ≈ dirty_channels / 1s
```

Например, четыре ноды и 300 активных каналов дают около 1200 shard updates/s
и до 300 cluster Occupancy events/s. WebSocket fan-out остаётся локальным и
равен числу подписчиков. Полный JSON frame кодируется один раз на channel,
а bounded queue хранит для Occupancy только последний snapshot; медленный
клиент не должен блокировать канал.

При штатном churn задержка счётчика не превышает одну секунду. После падения
ноды её shard сохраняется до lease expiry и удаляется reaper-ом целиком. При
начальных настройках lease `15s`, renew `5s` и reaper `1s` аварийный ghost
может сохраняться не более примерно `16s`.

## Вне v1

- REST `ChannelDetails`, получение статуса одного канала и перечисление каналов.
- `[meta]channel.lifecycle`.
- Occupancy integrations, webhooks и history.
- `objectPublishers`, `objectSubscribers` и LiveObjects.
- Полное Ably connection resume/recovery.
- Presence history.
- Exactly-once доставка.
- Синхронное подтверждение обработки события всеми realtime-нодами.
- Redis Cluster sharding.

## Обязательные инварианты

1. Redis является authoritative источником точных Presence attachments,
   members, `presence_revision` и materialized Occupancy counters. Анонимные
   guest attachments представлены в Redis абсолютными node shards и являются
   bounded-eventual aggregate с flush interval не более одной секунды.
2. Одна зафиксированная `presence_revision` создаёт ровно один canonical
   `PresenceChannelChanged`. Guest-only churn не создаёт Presence event на
   каждый attachment.
3. Точный Presence transition и запись canonical event в Redis outbox
   выполняются одной атомарной операцией. Guest shard flush и обновление
   Occupancy version также атомарны, но могут быть coalesced.
4. WebSocket `ACK` Presence mutation отправляется после Redis commit и durable
   outbox record, но не ждёт JetStream и другие ноды.
5. Исходная нода не выполняет параллельный direct broadcast: событие возвращается
   через её собственный JetStream consumer.
6. `ATTACH` не активируется, пока snapshot и buffered deltas не сведены по
   revision.
7. При недоступном Redis кластерный Presence не откатывается к process-local
   state. Guest shard может продолжать локальный учёт до bounded flush error,
   но нода становится `not ready` и не подтверждает stale aggregate.

## Целевая архитектура

```text
WebSocket action
      |
      v
PresenceService
      |
      v
PresenceStore
      |
      +-- Redis authoritative state
      +-- presence revision and occupancy version
      +-- guest shard flush / dirty channels
      `-- Redis Stream outbox
                  |
                  v
          PresenceOutboxPublisher
                  |
                  v
          NATS JetStream
             /    |    \
            v     v     v
        node-1  node-2  node-3
            |     |     |
            `-- local ChannelHub
```

Guest attachments additionally go through a local shard actor:

```text
guest ATTACH/DETACH
        |
        v
local GuestOccupancyShard --(<= 1s absolute flush)--> Redis Lua
        |                                                |
        `-- local Presence SYNC/deltas                 v
                                                  Occupancy publisher
                                                         |
                                                         v
                                                   JetStream snapshot
```

JetStream переносит уже зафиксированные canonical changes. Он не является
источником snapshot и не восстанавливает потерянный Redis state.

На каждой ноде работают supervised runtimes:

- `PresenceLeaseRenewer`;
- `PresenceReaper`;
- `PresenceOutboxPublisher`;
- существующий JetStream incoming consumer;
- локальный `OccupancyEmitter`.

Завершение обязательного runtime переводит кластерную ноду в `not ready`.

## Доменная модель

Внутри одного channel Ably identity участника — пара:

```text
(connection_id, client_id)
```

`application_id` и `channel` задают namespace хранения, но не являются частью
wire identity member-а. Полный storage key имеет вид:

```text
(application_id, channel, connection_id, client_id)
```

Целевая модель допускает несколько `clientId` одного connection. На первом
этапе обычное клиентское соединение может менять только member с `clientId` из
своего токена. Произвольный `clientId` разрешается только отдельной серверной
или wildcard-авторизацией.

Основные типы:

```rust
struct PresenceOwner {
    node_id: NodeId,
    boot_generation: BootGeneration,
}

struct PresenceMember {
    connection_id: ConnectionId,
    client_id: String,
    owner: PresenceOwner,
    data: Option<serde_json::Value>,
    last_message_id: String,
    presence_revision: u64,
    updated_at_ms: u64,
}

struct Attachment {
    connection_id: ConnectionId,
    owner: PresenceOwner,
    effective_modes: EffectiveChannelModes,
    occupancy: Option<OccupancySubscription>,
}

struct GuestOccupancyShard {
    owner: PresenceOwner,
    channel: ChannelKey,
    version: u64,
    connections: u64,
    subscribers: u64,
    presence_subscribers: u64,
    lease_deadline_ms: u64,
}

struct PresenceSnapshot {
    members: Vec<PresenceMember>,
    presence_revision: u64,
    occupancy: OccupancyMetrics,
}

struct CommittedTransition {
    presence_revision: Option<u64>,
    occupancy_version: u64,
    event_id: Option<Uuid>,
    duplicate: bool,
}

struct GuestShardResult {
    occupancy_version: u64,
    global_zero_boundary: bool,
    snapshot: OccupancyMetrics,
}
```

`PresenceAction::Present` используется только в `SYNC`. Canonical deltas имеют
действия `Enter`, `Update` или `Leave`.

## `PresenceStore`

Целевой контракт:

```rust
trait PresenceStore {
    async fn attach_and_snapshot(
        &self,
        command: AttachCommand,
    ) -> Result<AttachResult, PresenceStoreError>;

    async fn apply_presence(
        &self,
        command: PresenceBatchCommand,
    ) -> Result<CommittedTransition, PresenceStoreError>;

    async fn detach(
        &self,
        command: DetachCommand,
    ) -> Result<CommittedTransition, PresenceStoreError>;

    async fn disconnect(
        &self,
        command: DisconnectCommand,
    ) -> Result<Vec<CommittedTransition>, PresenceStoreError>;

    async fn snapshot(
        &self,
        channel: ChannelKey,
    ) -> Result<PresenceSnapshot, PresenceStoreError>;

    async fn flush_guest_shard(
        &self,
        shard: GuestOccupancyShard,
    ) -> Result<GuestShardResult, PresenceStoreError>;
}
```

Команда содержит:

- application и channel;
- connection и authorized client IDs;
- `node_id` и `boot_generation`;
- серверный timestamp;
- normalized request hash;
- stable operation ID;
- payload и effective modes, когда применимо.

Guest shard command не содержит member payload и не продвигает
`presence_revision`. Он содержит абсолютные counters и `version`; повторная
отправка той же или более старой version является no-op.

Один входной Presence `ProtocolMessage` обрабатывается как batch одного channel.
Все валидные элементы batch получают одну channel revision и один canonical
event. Частичный commit batch запрещён.

Клиентские retry дедуплицируются по:

```text
(application_id, connection_id, msg_serial)
```

Normalized payload хранится вместе с результатом. Повтор с тем же ключом:

- не меняет state второй раз;
- не увеличивает revision;
- не создаёт второй outbox record;
- возвращает сохранённый `ACK` или `NACK`.

Тот же ключ с другим normalized payload является protocol conflict. Для
внутренних cleanup-команд stable operation ID включает owner generation,
connection, channel и тип операции.

`PresenceService` использует этот operation ledger для любого Presence outcome,
включая precondition `NACK`, который можно идентифицировать по connection и
`msgSerial`. Сообщение без обязательного `msgSerial` отклоняется до mutation.
Ошибка декодирования, при которой эти поля получить невозможно, не может быть
дедуплицирована.

Outcome хранится до authoritative disconnect connection-а. Пока connection
жив, TTL ledger продлевается вместе с owner lease и не может истечь. Disconnect
и reaper помечают ledger закрытым, но удаляют его только после safety TTL, не
меньшего максимального поддерживаемого окна retry/resume. Иначе поздний повтор
того же `msgSerial` создаст новую revision.

`attach_and_snapshot` при повторном attach не увеличивает counters. Он атомарно
возвращает свежий snapshot и текущую revision, чтобы повторный ответ не
восстанавливал устаревший snapshot.

Будущие `InMemoryPresenceStore` и `RedisPresenceStore` проходят один contract
test suite. Memory-реализация нужна для автономного режима, но её внутреннее
устройство не определяет доменную модель.

## Redis authoritative model

Namespace:

```text
APP.APP_ENV.presence.v1
```

Логически Redis хранит:

- `presence_revision` и `occupancy_version`;
- attachments канала;
- members канала;
- materialized Occupancy metrics;
- guest occupancy shards `(node_id, boot_generation, channel)` с version;
- dirty-channel index и occupancy publish deadlines;
- reverse index connection -> channels и members;
- reverse index `(node_id, boot_generation) -> connections`;
- protocol operation results с TTL;
- node generation registry, deadline index и leases;
- Redis Stream outbox.

Application и channel в ключах кодируются length-prefixed или base64url
encoding. Необработанные имена channel не используются как структурные
разделители ключа.

Точный Presence Lua transition атомарно:

1. для клиентской команды проверяет действующий owner lease и точную
   `boot_generation`;
2. проверяет operation dedup и normalized request hash;
3. валидирует attachment и предыдущее member state;
4. меняет attachment/member и reverse indexes;
5. пересчитывает затронутые exact Occupancy counters;
6. при реальном Presence изменении увеличивает `presence_revision` ровно один раз;
7. формирует полный transport-independent record для одного
   `PresenceChannelChanged`;
8. выполняет `XADD` этого сообщения в outbox;
9. сохраняет operation outcome;
10. возвращает `presence_revision`, event ID и snapshot, если это attach.

Отдельный `flush_guest_shard` Lua transition атомарно:

1. проверяет lease и точную `boot_generation` shard-а;
2. отклоняет version, которая не новее сохранённой;
3. заменяет абсолютные guest counters shard-а;
4. применяет разницу к materialized Occupancy counters;
5. увеличивает `occupancy_version` только при изменении gauge;
6. помечает channel dirty и сохраняет ближайший publish deadline;
7. возвращает полный snapshot и факт глобального `0 ↔ >0`.

Обычный guest flush не создаёт Presence revision и отдельную outbox entry.
Dirty-channel publisher создаёт один coalesced occupancy event по deadline.

No-op и отклонённая команда не создают новую revision. Redis mutation с
последующим отдельным `EventBus::publish` запрещена: процесс может завершиться
между этими шагами.

Новый UUID генерируется приложением до вызова Lua и передаётся как кандидат
`event_id`. При первом commit Lua сохраняет его в outbox; при duplicate
operation кандидат игнорируется и возвращается исходный event ID. Outbox entry
содержит `event_name`, `schema_version`, event ID и полный payload, поэтому
publisher может восстановить тот же `EventMessage` без чтения mutable state.

V1 предполагает один логический Redis primary, на котором Lua имеет атомарный
доступ ко всем перечисленным ключам. Redis Cluster потребует отдельного дизайна
hash slots либо partitioned outbox.

Reaper не проходит через проверку lease уже умершего owner-а. Для него
существует отдельный fenced `reap_generation` transition: он проверяет lease
самого reaper-а, cleanup token, истёкший deadline целевой generation и точное
совпадение owner каждой удаляемой записи. Изменение channel state и outbox при
этом остаются атомарными и используют тот же canonical event format.

## Canonical Presence event и coalesced Occupancy event

Для каждой зафиксированной `presence_revision` создаётся ровно один canonical
`PresenceChannelChanged` с `DeliveryClass::AllNodes`. Точные identified
attachment transitions также могут содержать немедленный Occupancy snapshot
без продвижения Presence revision. Для guest-only churn создаётся latest-wins
Occupancy snapshot с отдельным `occupancy_version`.

```rust
struct PresenceChannelChanged {
    application_id: ApplicationId,
    channel: String,
    origin: PresenceOwner,
    presence_revision: Option<u64>,
    occupancy_version: u64,
    presence_deltas: Vec<PresenceDelta>,
    occupancy: Option<OccupancyChange>,
    occurred_at_ms: u64,
}

struct PresenceDelta {
    action: PresenceChangeAction,
    connection_id: ConnectionId,
    client_id: String,
    data: Option<serde_json::Value>,
    message_id: String,
    timestamp_ms: u64,
}

struct OccupancyChange {
    metrics: OccupancyMetrics,
    changed_categories: Vec<OccupancyCategory>,
    zero_boundary_categories: Vec<OccupancyCategory>,
}
```

Возможные варианты:

- `UPDATE` member-а: только `presence_deltas`;
- guest `ATTACH`/`DETACH`: только coalesced `occupancy`, без Presence revision;
- identified `ATTACH`: только `occupancy`;
- `ENTER` или `LEAVE`: delta и Occupancy snapshot в одном envelope;
- `DETACH`/disconnect: несколько leave deltas и Occupancy snapshot.

Это намеренно не две записи `PresenceChanged` и `OccupancyChanged` с одинаковой
revision. Presence consumer использует `presence_revision`, а Occupancy
consumer — latest-wins `occupancy_version`; guest churn не увеличивает
Presence cursor.

Occupancy передаётся полным gauge snapshot, а не `+1/-1`. `message_id`,
timestamp и payload Presence delta также назначаются до outbox, чтобы каждая
нода отправляла одинаковый wire payload.

Внутренняя Redis revision не является самостоятельным Ably wire field. Для
протокольной границы сервер формирует opaque `channelSerial`; полная Ably
recovery semantics в v1 не заявляется.

## Outbox и гарантии доставки

`PresenceOutboxPublisher` v1 имеет одного fenced leader-а и не более одной
in-flight entry. Каждая нода может запустить runtime, но Redis publish lease с
монотонным fence token разрешает claim только одному поколению. После failover
новый leader через `XAUTOCLAIM` сначала забирает pending entries в порядке
Stream ID и только затем читает новые `>` entries. Standby runtime считается
healthy, пока общий publish lease свежий. Параллелить publisher можно лишь после
partitioning, которое сохраняет порядок одного channel.

Leader проверяет fence перед каждым claim и publish и останавливается при
потере lease. Сам Redis lease не может физически отменить уже начавшийся сетевой
publish старого процесса, поэтому correctness дополнительно опирается на
stable event ID, `presence_revision`/`occupancy_version` и resync. Split-brain может дать duplicate или
редкий out-of-order delivery, но не расходящийся authoritative state.

Для каждой outbox entry:

1. Декодируется уже сформированный `EventMessage`.
2. Сообщение публикуется в JetStream с исходным `event_id`.
3. `event_id` передаётся как `Nats-Msg-Id`.
4. После JetStream publish ACK outbox entry получает `XACK` и может быть
   удалена.
5. Pending entry после падения worker-а забирается новым worker-ом.
6. Любой retry переиспользует те же поля `EventMessage` и event ID.

Outbox нельзя обрезать через approximate `MAXLEN` до publish ACK. Retention
удаляет только подтверждённые entries; иначе WebSocket ACK больше не означает
durable дальнейшую доставку.

Обычный `EventBus::publish` для outbox не подходит, потому что создаёт новый
event ID. Реализация должна добавить явный путь публикации подготовленного
`EventMessage` либо передавать его непосредственно transport publisher-у.

Гарантии ACK:

- Redis commit + outbox означает, что canonical change сохранено.
- WebSocket client `ACK` означает Redis commit и durable outbox record.
- JetStream publish ACK означает запись сообщения в stream.
- Consumer ACK означает обработку одной конкретной realtime-нодой.
- Ни один ACK не означает получение события всеми браузерами.
- Итоговая доставка — at-least-once, не exactly-once.

Один общий durable consumer недопустим: он превратил бы realtime-ноды в
competing consumers. Каждая нода использует собственный stable durable consumer.

Consumer:

1. декодирует envelope и проверяет schema/application;
2. выполняет dedup claim по `(node_id, event_id)`;
3. передаёт событие в локальный ordered channel projector;
4. буферизует его для pending attachments либо enqueue-ит локальным active
   recipients;
5. сохраняет complete dedup result;
6. отправляет transport ACK.

Повторный `event_id` успешно ACK-ается без повторного применения. Presence
revision ниже или равная уже применённой является stale no-op. Occupancy event с
устаревшим `occupancy_version` также является latest-wins no-op. Разрыв Presence
revision при наличии локальных recipients запускает Redis resync, а не молча
пропускается.
Отсутствие локальных recipients считается успешной обработкой.

Consumer cursor принадлежит event stream канала. Snapshot одного attachment не
продвигает этот cursor и не может скрыть event от уже active attachments.
Snapshot `presence_revision` хранится как barrier конкретного pending
attachment; `occupancy_version` хранится отдельно.

При gap локальный channel projector:

1. атомарно переходит в `Resyncing`;
2. буферизует все последующие events;
3. атомарно читает Redis snapshot вместе с `presence_revision`;
4. отправляет active presence subscribers corrective `SYNC`, а Occupancy
   subscribers — отфильтрованный полный snapshot;
5. отбрасывает buffered Presence events с revision не выше snapshot;
6. replay-ит более новые Presence events по revision;
7. обновляет channel cursor и возвращается в `Active`.

Dedup completion и consumer ACK выполняются только после успешного resync.
Ошибка snapshot/enqueue является retryable, даёт `NAK` и делает projection
unhealthy; cursor при этом не продвигается.

Retryable ошибка даёт `NAK`. Неизвестная schema или необратимо повреждённый
payload получают `TERM`/DLQ и переводят обязательную проекцию в unhealthy
состояние.

## ACK клиенту

Для Presence mutation в cluster mode:

```text
atomic Redis state + outbox commit
                  |
                  v
          WebSocket ACK
                  |
                  v
    asynchronous JetStream fan-out
```

Исходная нода не делает direct broadcast. Её клиенты получают delta через тот
же consumer, что и клиенты остальных нод.

При Redis error возвращается `NACK`; local `PresenceHub` не используется как
fallback. Потерянный ответ повторяется с тем же `msgSerial`, а сохранённый
outcome возвращается без нового commit.

Guest `ATTACH`/`DETACH` подтверждается после локальной регистрации или удаления
shard contribution. Global Occupancy commit выполняется следующим shard flush;
поэтому ACK guest attachment не обещает, что все ноды уже видят новое значение
`connections`. Initial snapshot имеет bounded staleness до одного секунды.

## `ATTACH`, `SYNC` и snapshot barrier

Target attach flow:

1. Проверить auth, capability, params и requested modes.
2. Создать локальный pending attachment и начать буферизовать события channel
   для этого connection.
3. Атомарно выполнить `attach_and_snapshot`.
4. Получить `{members, presence_revision, occupancy, effective_modes}` после регистрации
   authoritative attachment.
5. Подготовить единый упорядоченный ответ:
   - `ATTACHED` с recognized params, effective flags и opaque `channelSerial`;
   - `SYNC` с members в состоянии `Present`;
   - initial `[meta]occupancy`, если он запрошен.
6. Вызвать локальный `finish_attach`:
   - поставить `ATTACHED`, `SYNC` и initial Occupancy в outbound queue;
   - отбросить Presence events с `presence_revision <= snapshot.presence_revision`;
   - отсортировать и enqueue-ить более новые Presence events;
   - сохранить attachment barrier/cursor;
   - перевести attachment из `Pending` в `Active`.

Для guest attachment `attach_and_snapshot` не записывает connection в Redis и
не меняет `presence_revision`. Локальный actor увеличивает guest shard, а
`SYNC` строится только из Redis Presence members. Поэтому гость получает
актуальный список Presence без создания собственной member-записи. Initial
Occupancy может отражать shard flush с задержкой до одной секунды.

Pending barrier устанавливается до Redis transition. Поэтому delta, вошедшая в
snapshot, отбрасывается, а более новая delta всегда оказывается после `SYNC`.
Фильтрация выполняется только для этого attachment: те же события продолжают
доставляться другим active attachments.

`finish_attach` и projector сериализованы одним attachment actor либо lock.
Projector не может добавить event между последним drain и сменой состояния:
пока `finish_attach` владеет границей, event ждёт, а после `Active` идёт сразу в
outbound queue.

Initial Occupancy после `ATTACHED`/`SYNC` — собственная детерминированная
гарантия этого сервиса. Она не описывается здесь как отдельная публичная
гарантия Ably.

Если store operation не committed, pending attachment удаляется. Если commit
состоялся, но enqueue ответа завершился ошибкой, обычный idempotent disconnect
cleanup удаляет authoritative attachment.

## Leases, fencing и reaper

`REALTIME_NODE_ID` стабилен для ноды. Каждый старт процесса создаёт случайный
`boot_generation`. Owner любого attachment/member:

```text
(node_id, boot_generation)
```

Рекомендуемые начальные настройки:

- lease TTL: 15 секунд;
- renewal interval: 5 секунд;
- reaper poll interval: не более 1 секунды для guest shards.

Renewal продлевает lease Lua-скриптом только при совпадении ожидаемой
generation. Если непросроченный lease того же node ID принадлежит другой
generation, новый процесс не перезаписывает его и завершает startup.

Каждый state transition также проверяет lease и generation. Потерявшая lease
нода:

- прекращает новые attach/presence mutations;
- становится `not ready`;
- закрывает WebSocket sessions и завершает runtime до работы без fencing.

Redis хранит отдельный generation-to-connections index и ZSET generation
deadlines. Claim/renew атомарно используют Redis `TIME`, продлевают lease и
обновляют score конкретной `(node_id, boot_generation)`. ZSET entry не исчезает
вместе с TTL key, поэтому reaper имеет надёжный источник кандидатов.
Полагаться только на key expiration notifications нельзя.

Reaper:

1. находит просроченную generation;
2. получает bounded cleanup lock с монотонным fence token;
3. повторно по Redis `TIME` проверяет deadline и что текущий node lease не равен
   целевой generation;
4. обрабатывает connections ограниченными batch;
5. перед каждым batch проверяет cleanup token и удаляет только записи с точным
   owner match;
6. для exact members создаёт leave transitions через тот же Lua и outbox, а
   guest shards удаляет агрегированно и помечает channel dirty;
7. удаляет generation indexes после завершения.

Новая generation с тем же node ID не может быть удалена reaper-ом старой
generation и не блокирует cleanup старой: у каждой generation отдельный ZSET
member и owner index. Renewal старой generation, успевший до cleanup, сдвигает
deadline, а reaper обязан увидеть это при повторной проверке. Штатные `LEAVE`,
`DETACH`, disconnect и drain выполняют cleanup немедленно; lease expiry является
аварийной границей.

`WebsocketDisconnected` может оставаться `LocalOnly`: его локальный handler
вызывает authoritative disconnect transition. Межузловым является
`PresenceChannelChanged`, созданный этим transition.

## Occupancy v1

Authoritative counters:

| Категория | Определение |
|---|---|
| `connections` | Число unique connection attachments канала |
| `publishers` | Attachments с effective mode `publish` |
| `subscribers` | Attachments с effective mode `subscribe` |
| `presenceConnections` | Attachments с effective mode `presence` |
| `presenceSubscribers` | Attachments с effective mode `presence_subscribe` |
| `presenceMembers` | Число member records |

`publishers` не означает «connection хотя бы раз публиковал». Mode-based
метрики считаются из effective channel modes attachment-а.

Для guest attachment значения такие: `connections=1`, `subscribers=1`,
`presenceSubscribers=1` при явно запрошенном Presence subscribe mode, а
`publishers=0`, `presenceConnections=0` и `presenceMembers=0`. В materialized
gauge итоговые значения складываются из exact identified counters и всех
принятых guest shards. `connections` — это число channel attachments, а не
число уникальных людей или `clientId`.

Requested modes приходят в `ATTACH.flags`. Effective modes — пересечение
requested modes и token capabilities — возвращаются в `ATTACHED.flags`. Если
requested modes отсутствуют, применяются разрешённые сервером default modes.

Capability resolver должен поддерживать точное имя channel, namespace wildcard,
общий `*` и operation wildcard. Запрос Occupancy требует
`channel-metadata`. Пока capability checks и effective modes не реализованы,
Occupancy counters нельзя объявлять корректными.

`objectPublishers` и `objectSubscribers` не включаются в payload v1. Явный
запрос одной из этих категорий отклоняется как unsupported, а не возвращает
ложный ноль.

## Token authentication и Ably SDK

`profile=guest` — внутренний параметр прикладного API, но не параметр Ably
SDK. Для `authUrl` SDK отправляет `application/x-www-form-urlencoded`
параметры. Guest определяется отсутствующим `clientId` и capability, которая
ограничена сервером:

```text
capability={"chat:room":["channel-metadata","subscribe"]}
```

Запрос capability считается просьбой клиента. Сервер canonicalizes resource и
operations, проверяет точный channel и пересекает запрос с guest allowlist.
`publish`, `presence`, wildcard и чужие channels отклоняются. Браузер не может
передать `clientId`, TTL, `jti` или собственные JWT claims.

### Realtime SDK authUrl

```http
POST /auth/realtime/{application_id}/token
Content-Type: application/x-www-form-urlencoded
Accept: text/plain
```

Пример запроса:

```text
capability={"chat:room":["subscribe","channel-metadata"]}
```

При успехе endpoint возвращает сам JWT без JSON envelope:

```http
HTTP/1.1 200 OK
Content-Type: text/plain

<base64url-header>.<base64url-payload>.<base64url-signature>
```

JWT содержит `kid`, `iat`, `exp`, `jti`, `iss`, `aud`,
`x-ably-capability` и `x-realtime-token-kind=guest`. Claim
`x-ably-clientId` отсутствует. TTL guest token — номинально 10 минут; фактический
expiry получает jitter `±10%`, чтобы не создавать refresh storm.

Конфигурация клиента:

```ts
const realtime = new Ably.Realtime({
  authUrl: '/auth/realtime/app-id/token',
  authMethod: 'POST',
  authParams: {
    capability: JSON.stringify({
      'chat:room': ['subscribe', 'channel-metadata'],
    }),
  },
});
```

SDK повторяет запрос перед `exp` и может использовать ещё действующий JWT при
reconnect. Один token не является Presence member и не должен быть общим для
всех гостей: `jti` и active-connection lease позволяют ограничить replay.

Ошибки token endpoint используют стандартный HTTP status и короткий JSON
envelope `{ "data": { "message": "..." } }` для application API:

| Status | Причина |
|---|---|
| `400` | malformed form, unsupported capability или invalid channel |
| `403` | capability не входит в guest allowlist |
| `404` | неизвестный application или key |
| `429` | превышен rate limit; ответ содержит `Retry-After` |
| `503` | issuer или его dependency временно недоступны |

### Ably-compatible REST token endpoint для `ably-php-laravel`

`ably-php-laravel` является REST-only SDK. Он не открывает WebSocket и не
управляет guest Presence. Чтобы Laravel мог вызвать
`Ably::auth()->requestToken()`, сервер должен дополнительно поддержать
стандартный endpoint:

```http
POST /keys/{keyName}/requestToken
Authorization: Basic <keyName:keySecret>
Content-Type: application/json
```

PHP SDK сформирует Ably `TokenRequest` с canonical capability string:

```json
{
  "keyName": "app-id:key-id",
  "ttl": 600000,
  "capability": "{\"chat:room\":[\"channel-metadata\",\"subscribe\"]}",
  "timestamp": 1788278400000,
  "nonce": "random-value"
}
```

Guest token запрашивается из Laravel так:

```php
$tokenDetails = Ably::auth()->requestToken([
    'capability' => [
        'chat:room' => ['subscribe', 'channel-metadata'],
    ],
    'ttl' => 600000,
]);
```

`clientId` для guest не передаётся. Rust возвращает стандартный TokenDetails:

```json
{
  "token": "<jwt>",
  "issued": 1788278400000,
  "expires": 1788279000000,
  "capability": "{\"chat:room\":[\"channel-metadata\",\"subscribe\"]}"
}
```

Оба endpoint используют один issuer и одну guest policy. Отдельный
`/guest-token` не нужен. Если стандартный `/keys/{keyName}/requestToken` ещё не
реализован, Laravel вызывает application endpoint обычным HTTP-клиентом и
передаёт полученный `data.jwt` в свой `authUrl` как plain text. `ably-php-laravel`
в таком режиме не участвует в выдаче токена. Для
`Ably::auth()->requestToken()` SDK должен быть настроен на наш REST host; иначе
он отправит запрос в Ably Cloud.

### Token load

Выдача JWT не выполняет Presence transition и не пишет guest attachment в
Redis. При TTL 10 минут устойчивый refresh rate приблизительно равен
`active_connections / 600` плюс rate новых подключений. Для 30 000 активных
гостей это около 50 refresh requests/s до учёта churn. Rate limiting выполняется
отдельно по application, channel и trusted client IP; ошибки capacity возвращают
`429` с `Retry-After` или `503`.

## Realtime wire contract

Клиент запрашивает Occupancy через `ATTACH.params`:

```json
{
  "action": 10,
  "channel": "chat:room",
  "params": {
    "occupancy": "metrics"
  }
}
```

Поддерживаемые v1 values:

```text
metrics
metrics.connections
metrics.publishers
metrics.subscribers
metrics.presenceConnections
metrics.presenceSubscribers
metrics.presenceMembers
```

Невалидное/unsupported значение или отсутствие `channel-metadata` отклоняет
attach с protocol error.

`ATTACHED.params` содержит только распознанный и проверенный canonical subset.
Если params не запрашивались или ни один не распознан, семантически это `{}`:

```json
{
  "action": 11,
  "channel": "chat:room",
  "params": {
    "occupancy": "metrics"
  }
}
```

Requested modes находятся в `ATTACH.flags`; granted/effective modes — в
`ATTACHED.flags`.

Occupancy передаётся обычным `MESSAGE`. Ниже показан raw JSON wire payload:

```json
{
  "action": 15,
  "channel": "chat:room",
  "messages": [
    {
      "name": "[meta]occupancy",
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "timestamp": 1788278400000,
      "data": "{\"metrics\":{\"connections\":10,\"publishers\":3,\"subscribers\":9,\"presenceConnections\":8,\"presenceSubscribers\":7,\"presenceMembers\":12}}",
      "encoding": "json"
    }
  ]
}
```

`id` и `timestamp` назначаются сервером. `clientId` и `connectionId` у
`[meta]occupancy` не задаются. После обработки `encoding: "json"` SDK видит
обычный объект `data.metrics`. Для `metrics.<category>` объект `metrics`
содержит только выбранный ключ.

После initial snapshot:

- пересечение нулевой границы `0 <-> non-zero` любой включённой категорией
  отправляется немедленно;
- остальные изменения coalesce до последнего snapshot;
- debounce не превышает 15 секунд;
- подписка `metrics` реагирует на любую из шести категорий;
- category subscription реагирует только на выбранную категорию;
- debounce хранится только локально и не является authoritative state;
- detach подписчика отменяет его pending delivery.

Deadline отсчитывается от первого отложенного изменения и не переносится
последующими updates. Немедленная zero-boundary delivery отправляет последний
полный snapshot выбранного профиля и завершает накопленный debounce batch.

Presence messages, созданные клиентским batch, получают стабильный ID на основе
`connectionId`, `msgSerial` и batch index в формате
`connectionId:msgSerial:index`. Internal cleanup получает стабильный
server-generated ID из committed event в формате `server:eventId:index`. ID
synthesized `LEAVE` не должен начинаться с `connectionId` удаляемого member-а:
это не даёт SDK ошибочно разобрать его как client operation и оставляет
timestamp границей порядка. Все ноды используют ID из canonical event и не
генерируют его при локальной доставке.

## Конфигурация и runtime boundary

Целевые, ещё не существующие настройки:

```env
# autonomous
EVENT_BUS_DRIVER=local
PRESENCE_STORE_DRIVER=memory

# clustered Presence
EVENT_BUS_DRIVER=nats
PRESENCE_STORE_DRIVER=redis
REALTIME_NODE_ID=realtime-550e8400-e29b-41d4-a716-446655440000
```

Допустимые production-профили:

| Event bus | Presence store | Статус |
|---|---|---|
| `local` | `memory` | Автономный single-node |
| `nats` | `redis` | Кластерный |
| `nats` | `memory` | Только обычные cluster messages; cluster Presence запрещён |
| `local` | `redis` | Не поддерживается v1 |

Неизвестное или несовместимое значение завершает startup. При runtime error нет
автоматического fallback.

Readiness кластерной ноды учитывает:

- Redis connectivity и возможность fenced transition;
- lease freshness;
- outbox publisher health и oldest pending age;
- NATS/JetStream publisher;
- incoming consumer loop и consumer lag policy;
- reaper health;
- отсутствие permanent projection error.

## Failure model

| Сбой | Поведение |
|---|---|
| Redis недоступен до commit | Mutation получает `NACK`; local fallback запрещён |
| Ответ Redis потерян после commit | Retry с тем же `msgSerial` возвращает сохранённый outcome |
| Процесс упал после commit до publish | Outbox сохраняется; другой publisher продолжает доставку |
| NATS недоступен | Outbox накапливается, retry сохраняет event ID, readiness деградирует |
| Падение после publish ACK до outbox ACK | Возможен повтор publish; `Nats-Msg-Id` и consumer dedup делают его безопасным |
| Consumer упал после local enqueue до ACK | JetStream повторяет event; event dedup/revision защищают повторное применение |
| Обнаружен gap revision | Активные локальные проекции resync-ятся из Redis |
| Guest shard flush повторён или пришёл не по порядку | Redis принимает только более новую shard version; counters не удваиваются |
| Нода упала с активными гостями | Reaper удаляет весь shard после lease expiry; временный ghost ограничен TTL |
| Guest churn внутри flush window | В Redis и JetStream попадает последний полный gauge, а не event на каждый connect/disconnect |
| Guest token refresh storm | TTL jitter распределяет expiry; rate limiter возвращает `429` с `Retry-After` |
| Два reaper-а | Cleanup lock, operation ID и generation checks делают cleanup идемпотентным |
| Переиспользован node ID | Fencing не даёт поколениям менять или удалять чужой state |
| Redis state полностью потерян | Presence/Occupancy не восстанавливаются из JetStream; sessions закрываются и входят заново после восстановления |
| Permanent event error | `TERM`/DLQ и unhealthy projection; событие не пропускается молча |

Redis replication durability остаётся инфраструктурным требованием. Outbox не
защищает от потери уже подтверждённой primary-записи при неверно настроенном
Redis failover.

## Этапы реализации

1. Добавить domain types и `PresenceStore`; перенести текущую local-логику в
   memory adapter и общий contract test.
2. Ввести capability resolver, requested/effective channel modes и
   authoritative attachment model.
3. Реализовать Redis schema, Lua transitions, protocol dedup, revisions и
   durable outbox, включая guest shards и dirty-channel index.
4. Добавить `PresenceChannelChanged`, публикацию prepared `EventMessage` и
   ordered handler `AllNodes`.
5. Перевести `ENTER`, `UPDATE`, `LEAVE`, `DETACH` и disconnect на store; убрать
   direct broadcast.
6. Реализовать pending attach barrier, opaque `channelSerial` и resync при
   revision gap.
7. Добавить `boot_generation`, lease renewer, fencing и reaper.
8. Расширить wire types: `params`, modes, `encoding` и metadata обычного
   `Message`.
9. Реализовать шесть Occupancy metrics, guest shard flush и local debounce
   emitter.
10. Ввести guest policy и capability enforcement для `authUrl`, а также
    стандартный `/keys/{keyName}/requestToken` adapter для PHP SDK.
11. Подключить runtimes к composition root, readiness и observability.
12. Выполнить live two-node failure smoke до включения cluster Presence в
    production.

Этапы 2 и 3 могут разрабатываться параллельно, но cluster Presence нельзя
включать до завершения обоих.

## Тесты и acceptance criteria

### Store contract

- enter/update/leave и mixed batch;
- несколько client IDs одного connection на уровне store;
- повтор `msgSerial` возвращает прежний outcome и не меняет revision;
- тот же `msgSerial` с другим payload отклоняется;
- duplicate attach не удваивает counters;
- detach удаляет members attachment-а;
- disconnect очищает все channels connection-а;
- snapshot содержит только `Present`;
- counters не уходят ниже нуля;
- одна `presence_revision` создаёт один Presence event;
- повторный guest shard flush не создаёт новую Presence revision;
- Redis state и outbox выполняются вместе либо не выполняются.
- outbox leader failover сначала обрабатывает pending и сохраняет commit order.

### Snapshot race

- commit до snapshot;
- commit во время attach;
- commit после snapshot;
- duplicate/redelivery;
- пустой snapshot;
- outbound queue failure во время attach;
- revision gap и Redis resync.
- snapshot одного attachment не продвигает cursor других attachments.

После `SYNC` клиент имеет authoritative set без пропусков и delta до snapshot.

### Две ноды

- enter на node A виден клиентам A и B;
- snapshot на B содержит member, созданный на A;
- update/leave распространяются на обе ноды;
- origin node получает event только через своего consumer-а;
- restart consumer-а вызывает безопасную redelivery;
- отсутствие локальных recipients успешно ACK-ается.

### Lease и reaper

- graceful close даёт немедленный leave;
- crash очищается после lease expiry;
- renewal не продлевает чужую generation;
- два reaper-а не создают duplicate leave;
- новая generation прежнего node ID не удаляется старым cleanup;
- потерявшая lease нода не выполняет mutations.

### Occupancy

- каждая из шести метрик проверяется отдельно;
- capabilities и requested modes дают правильное пересечение;
- `presenceMembers` может быть больше `presenceConnections`;
- initial snapshot включает attachment самого подписчика;
- category selector не содержит лишних полей;
- любая включённая zero-boundary отправляется немедленно;
- прочие изменения coalesce и отправляются не позднее 15 секунд;
- Objects category получает явную unsupported error.

### Guest high churn

- guest attachment виден в `connections` и `subscribers`, но не в
  `presenceMembers`;
- guest получает `SYNC` и Presence deltas, но `ENTER`/`UPDATE`/`LEAVE` и
  publish получают `NACK`;
- 10 000 connect/disconnect transitions внутри flush window дают bounded
  Redis shard writes, а не 10 000 outbox entries;
- stale/duplicate shard version не меняет counters;
- общий переход `0 ↔ >0` публикуется немедленно;
- обычный Occupancy update доставляется не реже одного раза в секунду при
  изменении gauge;
- падение ноды удаляет guest shard после lease expiry без тысяч отдельных
  Presence `LEAVE`.

### Token HTTP contract

- `authUrl` принимает form-encoded `capability` и возвращает raw JWT
  `text/plain`;
- capability canonicalized и ограничивается guest allowlist;
- `clientId`, `publish`, `presence`, wildcard и чужие channels отклоняются;
- JWT содержит `kid`, `iat`, `exp`, `jti`, `iss`, `aud` и
  `x-ably-capability`, но не `x-ably-clientId`;
- `/keys/{keyName}/requestToken` принимает стандартный Ably TokenRequest и
  возвращает TokenDetails;
- PHP `Ably::auth()->requestToken()` и browser `authUrl` используют один
  issuer и одинаковую guest policy;
- истёкший токен не принимается, а duplicate `jti` ограничивается active
  connection policy.

### Failure tests

- Redis outage без memory fallback;
- NATS outage с сохранением outbox;
- crash между commit и publish;
- crash между publish ACK и outbox ACK;
- consumer redelivery;
- DLQ/permanent failure;
- полная потеря Redis state;
- live two-node test с реальными Redis и JetStream.

## Наблюдаемость

Минимальные метрики:

- attachments, members и Occupancy gauges по application;
- Redis transition latency/errors;
- outbox length, oldest age и publish retries;
- JetStream publish ACK latency;
- consumer lag, redelivery и dedup hits;
- lease renewal age/errors;
- reaper generations/connections/leaves;
- snapshot size/latency;
- attach-barrier buffered deltas и resync;
- Occupancy immediate/debounced emissions;
- guest shard flush rate, stale versions и lease age;
- guest token issue/refresh rate, `429`, `503` и expiry jitter;
- Occupancy fan-out rate, coalesced frames и dropped slow consumers.

Логи содержат `application_id`, channel hash, `event_id`, `revision`, `node_id`
и `boot_generation`, но не полный channel name или Presence payload.

## Официальные источники

- [Ably: Presence and Occupancy](https://ably.com/docs/presence-occupancy)
- [Ably: Presence](https://ably.com/docs/presence-occupancy/presence)
- [Ably: Occupancy](https://ably.com/docs/presence-occupancy/occupancy)
- [Ably: Channel options and Occupancy params](https://ably.com/docs/channels/options)
- [Ably: JWT authentication](https://ably.com/docs/auth/token/jwt)
- [Ably: REST API Token Request specification](https://ably.com/docs/api/token-request-spec)
- [Ably PHP Laravel SDK](https://github.com/ably/ably-php-laravel)
- [Ably features specification](https://github.com/ably/specification/blob/main/specifications/features.md)
