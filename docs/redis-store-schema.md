# Redis Presence: схема ключей v1

Статус: реализованы store, сериализация, Lua transitions и runtime точного
Presence. Ключи aggregated Occupancy зарезервированы для отдельного этапа.

Связанные документы: [подзадача интеграции](redis-store-integration.md),
[доменный дизайн](presence-occupancy.md).

Контракты операций: [attach_and_snapshot](redis-attach-and-snapshot.md),
[snapshot](redis-snapshot.md), [apply_presence](redis-apply-presence.md),
[detach](redis-detach.md), [disconnect](redis-disconnect.md).

## Структура адаптера

`crates/realtime/src/channel/store/redis/`:

- `mod.rs` — граница модуля, экспорт `RedisKeys`;
- `keys.rs` — полные Redis-ключи, без I/O и чтения окружения;
- `protocol.rs` — версия схемы и кодирование идентификаторов; сюда будут
  добавлены сериализация значений и декодирование ответов transitions;
- `store.rs` — место будущей реализации `RedisChannelStore` и store-трейтов;
- `scripts/` — место будущих Lua transitions;
- `tests.rs` — стабильность схемы, изоляция и отсутствие коллизий.

На этом этапе нет конструктора store или методов с `todo!()`. Runtime остаётся
на `MemoryChannelStore`.

## Namespace и кодирование

`P = APP.APP_ENV.presence.v1`. Построитель `RedisKeys::new(APP, APP_ENV)` использует
`support::app::AppNamespace`; окружение читает будущий composition root.
APP и APP_ENV — deployment namespace, application ID — отдельная tenant-граница.

Все динамические строки кодируются как `B(s) = base64url(UTF-8(s))` без padding.
Unicode не нормализуется; пустая строка кодируется в пустой сегмент и остаётся
отличимой при фиксированном числе сегментов. Алфавит B не содержит `.`, `:`,
Redis glob-символов и hash tags `{}`. Имена не подставляются в ключи напрямую.

Обозначения:

- `A = B(application_id)`, `C = B(channel)`, `K = B(connection_id)`;
- `U = B(client_id)`, `N = B(node_id)`;
- `G = N.<boot_generation UUID>`; UUID — каноническая lowercase hyphenated
  строка, `started_at` не входит в идентичность;
- ссылка на канал — `A.C`, соединение — `A.K`, member канала — `K.U`;
- ссылка на member внутри connection index — `C.U`;
- `S` — msg_serial как десятичная строка шириной 20 символов с ведущими нулями.

Составные ссылки в HASH/SET/ZSET используют те же сегменты. Для разбора нужно
сохранять пустые сегменты. Они не являются полными Redis-ключами.

Пример: application `app`, channel `room`:
`webapp.test.presence.v1.app.YXBw.channel.cm9vbQ.state`.

## Ключи данных

Все перечисленные ключи строятся `RedisKeys`. Redis types и поля ниже —
контракт для будущих transitions, а не уже существующие записи runtime.

| Ключ после `P.` | Redis type | Содержимое |
|---|---|---|
| `app.A.channel.C.state` | HASH | `presence_revision`, `occupancy_version` и шесть counters: `connections`, `publishers`, `subscribers`, `presence_connections`, `presence_subscribers`, `presence_members` |
| `app.A.channel.C.attachments` | HASH | Field `K` → attachment с generation, effective modes и occupancy subscription |
| `app.A.channel.C.members` | HASH | `K.U` → подготовленный Rust JSON MemberPayload; `K.U:revision` и `K.U:updated_at` → числовые metadata |
| `app.A.channel.C.shards` | HASH | Field `G` → абсолютные counters, shard version и deadline |
| `app.A.connection.K.state` | HASH | `generation`, `status` (`open`/`closed`), `highest_serial` при наличии операций, `closed_at_ms` после закрытия |
| `app.A.connection.K.channels` | SET | `C` для точных attachments |
| `app.A.connection.K.members` | SET | `C.U` для members соединения |
| `app.A.connection.K.operations.S` | HASH | Одна операция: fingerprint, result и полные поля исходного события либо отказа; контракт в [apply_presence](redis-apply-presence.md) |
| `app.A.connection.K.operation-order` | ZSET | Member `S`, score всегда `0`: лексикографический порядок для удаления минимальных serial при переполнении ledger |
| `generations` | HASH | Field `G` → metadata запуска, включая `node_id`, `boot_generation`, `started_at` |
| `generation-deadlines` | ZSET | Member `G`, score — deadline в миллисекундах Redis TIME |
| `generation.G.connections` | SET | `A.K` для открытых соединений, включая ledger без текущих attachments; закрытые удаляются из SET и очищаются TTL |
| `generation.G.shards` | SET | `A.C` для aggregated shards, включая нулевые shards с сохранённой version |
| `outbox` | STREAM | Полные неизменяемые данные canonical event: event name, schema version, исходный event ID, canonical timestamp и данные payload; внутренний формат версионируется отдельно |
| `dirty-channels` | ZSET | Member `A.C`, score — ближайший Occupancy publish deadline |
| `occupancy-publications` | HASH | Field `A.C` → claim версии Occupancy: version, deadline, token publisher-а и сохранённый snapshot |

Versions, fence и msg_serial хранятся без потери целочисленной точности. `S`
используется в суффиксе HASH-ключа операции и в ZSET, поэтому serial не преобразуется в double score.
`highest_serial` хранится в том же формате. Deadline scores — Unix milliseconds,
назначаемые Redis; это не номера revisions или операций.

Шесть счётчиков канала, `presence_revision` и `occupancy_version` имеют диапазон
`0..9 007 199 254 740 991` (`2^53 - 1`), чтобы арифметика Lua оставалась точной.
В Redis они хранятся каноническими десятичными строками, в Rust — как `u64`.
Предел проверяется до записи; переполнение не сбрасывает значение. Это
ограничение не меняет форматы и диапазоны fence, msg_serial и Redis Stream ID.

Snapshot читает state и members атомарно. Любой transition поддерживает прямые
и обратные indexes в той же операции. Переданные клиентом application/channel
не позволяют получить state другого application.

## Lease-ключи

| Ключ после `P.` | Ресурс |
|---|---|
| `lease.node.N` | Один активный запуск данного node ID для всех applications |
| `lease.publisher` | Единственный publisher общего outbox |
| `lease.cleanup.G` | Cleanup конкретной целевой generation |

Значение и TTL этих ключей принадлежат протоколу `redis-lease`:
`lease:<owner>:<fence>`. Для каждого lease крейт также занимает **полный ключ
lease с добавленным `:fence`**, тип STRING, без TTL.

Построитель не принимает произвольный lease key и не строит fence-ключи сам.
Ни один ключ данных не содержит `:`, а все lease-ресурсы находятся под
`P.lease.`. Поэтому ни пользовательский channel `x:fence`, ни другой вид
ресурса не может занять служебный счётчик.

Node lease зависит только от node ID: новый boot конкурирует за тот же ключ.
Generation indexes и cleanup lock зависят от node ID + boot generation.
Владелец cleanup lease — reaper, а generation в ключе — цель очистки.

## Жизненный цикл и удаление

| Данные | TTL и правило удаления |
|---|---|
| Channel state | Без TTL. Даже пустой канал сохраняет versions: позднее событие или новый attach не должны увидеть сброс revision. Удаление возможно только при контролируемом выводе всего namespace из эксплуатации |
| Attachments и members | Без TTL. Удаление только authoritative detach/disconnect/reaper transition с counters, indexes и outbox; TTL не должен молча удалить участника без Leave |
| Shards | Без TTL Redis-ключа. Deadline хранится в записи; очистка generation атомарно вычитает contribution. Нулевой shard сохраняет version до cleanup, иначе запоздалый flush мог бы восстановить старый вклад |
| Open connection state и ledger | Без TTL, размер ledger ограничен `presence_ledger_capacity`. Запись соединения находится в generation index; reaper обеспечивает cleanup после смерти ноды |
| Closed connection state, каждый operations.S, operation-order | Первый disconnect/reaper задаёт общий `closed_at_ms` и абсолютный срок удаления `closed_at_ms + connection_state_ttl`. Повтор закрытия не продлевает срок. Retention не меньше поддерживаемого окна retry/resume |
| Connection channels/members | Удаляются при соответствующих transitions; после завершения disconnect пусты. Detach последнего канала не закрывает ledger живого соединения |
| Generation metadata, deadlines и indexes | Без TTL. Reaper удаляет только после завершения всех batch, закрытия ledger, удаления members/attachments и вычитания shards этой generation |
| Dirty channels и publication claims | Без TTL. Dirty marker удаляется только после outbox commit нужной версии и проверки, что более новая version не требует публикации. Claim старого publisher-а подлежит восстановлению после потери его lease |
| Outbox | Без TTL и без approximate MAXLEN. Неподтверждённые/pending entries сохраняются. Удаление entry разрешено после JetStream publish ACK и подтверждения обработки в outbox consumer group |
| Lease | TTL назначается claim/renew; node lease и generation deadline обновляются атомарно |
| Fence counters | Без TTL; release, истечение lease и cleanup generation их не удаляют. Счётчик нельзя сбрасывать, пока namespace может принимать старые tokens |

Отсутствие TTL у открытого ledger — намеренное решение v1: node renewal не
проходит по всем connection keys, а dedup не исчезает раньше authoritative
disconnect. При закрытии A.K удаляется из generation index, затем применяется retention. При смерти ноды записи
сначала обнаруживаются через generation index, затем закрываются reaper-ом.

Цена сохранения монотонности — retained channel metadata и fence counters,
в том числе counters cleanup locks завершённых generations. Их количество
нужно учитывать; обычный reaper не выполняет небезопасный GC этих счётчиков.

## Версионирование и атомарная граница

V1 рассчитана на один логический Redis primary. Hash tags для Redis Cluster
не добавляются: разбиение по slots потребует отдельного дизайна multi-key
transitions и outbox.

Переименование ключа/поля, изменение Redis type, кодирования идентичности или
смысла сохранённого значения требует явной миграции либо нового namespace
`presence.v2`. Приватность Rust-функции не отменяет совместимость данных.
Добавлять необязательные поля внутри v1 можно только при совместимости всех
одновременно работающих readers/writers. Версия canonical event schema
независима от версии ключей.

При смене namespace требуется согласованный drain старых sessions и workers,
доставка старого outbox и fencing старых writers. Простое переключение
`v1 → v2` не является безопасной миграцией и сбрасывает локальные счётчики
нового namespace. Старый namespace удаляется только после завершения этого
процесса; dual-write автоматически не предполагается.

Для каждого будущего Lua transition сначала перечисляются KEYS и проверки,
затем записи state/indexes/ledger/versions/outbox. Runtime error не откатывает
уже выполненные записи: наличие namespace и Lua само по себе ещё не доказывает
согласованность commit. Эти гарантии проверяются на этапе transitions.

Формат members и отдельные HASH операций заменяют прежний экспериментальный
JSON-формат. Чтение старых экспериментальных записей не поддерживается.
Для проверки новой схемы нужен чистый изолированный namespace. Форматы outbox
`attach.v2`/`presence.v2` версионируются отдельно от публичного события (schema 1).
