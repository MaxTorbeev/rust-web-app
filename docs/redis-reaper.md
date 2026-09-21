# Redis: очистка истёкших поколений

Реализованы поиск кандидатов `RedisChannelStore::expired_generations` и очистка
одного соединения `RedisChannelStore::reap_connection`, использующая общий с
disconnect код закрытия. `reap_generation_batch` обходит connections и финализирует
registry; `RedisPresenceRuntime` запускает фоновый цикл. Проверки включают
сохранение нового поколения и очистку после аварийного завершения ноды.

## Поиск кандидатов

`expired_generations(limit: u32)` возвращает `Vec<NodeInstance>`. Один Lua-вызов:

1. Получает Redis TIME в миллисекундах.
2. Выбирает до limit записей с `deadline <= now` из generation-deadlines через
   ZRANGEBYSCORE с LIMIT, по возрастанию deadline.
3. Читает metadata выбранных поколений из generations через HGET.

KEYS: generation-deadlines, generations; ARGV: limit. Ответ — массив пар
`{generation, metadata_json}`. Lua передаёт JSON без декодирования; Rust разбирает
NodeInstance и проверяет соответствие node ID + boot идентификатору поколения.
Отсутствующие metadata, неверный JSON и несовпадение identity возвращают Internal.
При limit=0 или отсутствии кандидатов результат пустой.

Метод не проверяет lease вызывающей ноды, не захватывает cleanup lock и ничего
не меняет. Список — кандидаты: к началу очистки deadline мог быть продлён,
поэтому reap_connection повторно проверяет deadline и leases. Limit ограничивает
размер выборки, а не число допустимых нод или посетителей.

## Вызов и владение

Store принадлежит живой ноде reaper-а. `DisconnectConnectionCommand.actor`
указывает соединение умершей ноды, а `event_id` служит базой событий каналов.
JSON и производные event ID, как в disconnect, готовятся в Rust.

Вызывающий получает cleanup token через `RedisLease::acquire`:

- ключ — `RedisKeys::cleanup_lease(target)`;
- владелец — `node_lease_owner(reaper_instance)`;
- TTL — ограниченный срок владения, продлеваемый через `RedisLease::renew`.

Rust проверяет соответствие ключа целевому поколению и владельца текущему
reaper-у. Lua перед чтением каналов и перед commit заново проверяет:

1. Полный node token reaper-а и его непросроченный generation deadline.
2. Полный cleanup token, включая fence, и его принадлежность reaper-у.
3. Наличие deadline цели и `deadline <= Redis TIME`.
4. Текущий node lease цели не принадлежит удаляемому поколению. Lease нового
   boot с тем же node ID допустим и не изменяется.

Проверки и очистка выполняются в одном Lua-вызове. Между предварительным
получением списка каналов и commit владение могло измениться: следующий вызов
повторяет все проверки. Время событий и retention берётся из Redis TIME.
При неопределённом результате renewal нельзя начинать следующий cleanup до
подтверждения владения; автоматического reacquire внутри метода нет.

## KEYS и ARGV

Основа — [контракт disconnect](redis-disconnect.md), с такими отличиями:

- KEYS[1] — node lease reaper-а; KEYS[2] — общий generation deadlines.
- KEYS[3..9] относятся к удаляемому соединению, его поколению и общему outbox.
- KEYS[10] — cleanup lease целевого поколения.
- KEYS[11] — node lease целевой ноды.
- После 11 базовых ключей идут state/attachments/members каждого канала.
- ARGV[1] — node token reaper-а; ARGV[2] — целевое поколение.
- ARGV[3..8] сохраняют смысл disconnect: приложение, connection, JSON целевого
  NodeInstance, retention и каналы.
- ARGV[9] — поколение reaper-а; ARGV[10] — полный cleanup token.

ARGV формирует типизированный Rust-адаптер. Удаляемые connection state,
attachments и members должны принадлежать точно заданным node ID + boot.
Несовпадение останавливает очистку до первой записи.

## Закрытие и повтор

Единица очистки — одно соединение со всеми его каналами. Поколение целиком не
загружается в один Lua-вызов. Работа внутри соединения пока не ограничена числом
каналов или участников: это не гарантия максимального времени исполнения Lua.

`close_connection.lua` используется и disconnect, и reaper: проверяет все
каналы и ledger, записывает canonical события `detach.v1`, удаляет attachments
и members, обновляет counters, закрывает ledger и назначает retention. Origin
события сохраняет NodeInstance удаляемого соединения, как в обычном disconnect.
Сам node lease цели, generation metadata и deadline этот transition не удаляет.

После commit A.K удалён из generation connections. Если его там уже нет,
reaper возвращает пустой результат без записей, в том числе после истечения
retention. Повтор не продлевает TTL и не создаёт новый tombstone или Leave.
Результат SISMEMBER передаётся в общий helper закрытия без повторного чтения.
Если этот connection ID уже принадлежит другому поколению, его записи не
затрагиваются. При оставшейся ссылке старого поколения несовпадение владельца
будет ошибкой.

Ответ разбирается существующим `decode_disconnect`. `generation_active`,
`lease_lost` и `generation_mismatch` становятся `Conflict`; это повод остановить
текущий проход и заново оценить владение. Потеря ответа не доказывает отсутствие
commit: доставка уже сохранённых событий выполняется через outbox. Ограничения
Lua при runtime error те же, что описаны в контракте disconnect.

## Обход и финализация

Runtime захватывает cleanup lease на 15 секунд и запускает одну порцию до 32
соединений с таймаутом 10 секунд. Затем освобождает lease. Lua выбирает ссылки
через SRANDMEMBER; каждая очистка повторно проверяет node/cleanup leases и deadline.
При обрыве прохода оставшиеся ссылки будут выбраны следующим reaper-ом.

Если индекс connections пуст, `reap_generation.lua` проверяет отсутствие shards
и удаляет metadata, deadline и пустые индексы поколения. Fence counters и lease
нового запуска не затрагиваются. Финализация защищена теми же проверками.
Непустой shards index пока возвращает ошибку: aggregated Occupancy cleanup
будет реализован вместе с shard store.
