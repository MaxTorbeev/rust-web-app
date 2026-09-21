# Redis: disconnect

`RedisChannelStore::disconnect` удаляет соединение из всех его каналов и закрывает
Presence ledger. Подготовка одного канала и commit переиспользуются с detach;
`close_connection.lua` также обслуживает [очистку reaper-а](redis-reaper.md).
Метод подключён через `AttachmentStore` к runtime. Проверены live Redis и
доставка Leave подписчику другой ноды при закрытии WebSocket.

## Подготовка в Rust и атомарный commit

Команда содержит только actor и базовый event ID, а список каналов хранится
в Redis. JSON каналов и производные event ID готовит Rust:

1. Первый вызов `DISCONNECT` передаёт пустой список каналов. Lua проверяет lease,
   connection state и generation index, читает connection channels.
2. Если набор каналов отличается от переданного, Lua возвращает
   `{"channels", segments}` без записей. Rust декодирует base64url/UTF-8, сортирует
   исходные имена, строит ChannelKey, JSON и `command.channel_event_id(channel)`.
3. Повторный вызов проверяет точное совпадение наборов. При конкурентном attach
   или detach Lua снова возвращает актуальный список, Rust повторяет подготовку.
4. Когда набор совпал, Lua читает и проверяет текущее состояние всех каналов,
   готовит все удаления и события, затем выполняет commit без промежуточных
   запросов. Если канал удалили и присоединили заново между запросами, используется
   его актуальное состояние из последнего Lua-вызова.

Обычно нужны два запроса. Для соединения без каналов или уже закрытого — один.
Первое чтение не резервирует каналы и не меняет состояние. На ошибке транспорта
автоматического повтора нет; вызывающий код получает Internal. При непрерывном
изменении набора каналов подготовка может повторяться до прекращения гонки
или отмены вызывающей задачи.

## KEYS и ARGV

Базовые KEYS:

1. Node lease.
2. Generation deadlines.
3. Connection state.
4. Connection channels.
5. Connection members.
6. Generation connections.
7. Outbox.
8. Connection operation order.
9. Префикс connection operations (сам ключ не содержит данных).

После них для каждого канала в порядке входного массива — три ключа:
channel state, attachments, members. Ключи operations.S для retention строятся
из KEYS[9] и проверенных serial. Текущая схема рассчитана на standalone Redis.

ARGV:

1. Полное значение lease token.
2. Generation G.
3. Сегмент приложения A.
4. Сегмент соединения K.
5. Исходный connection ID.
6. Исходный JSON NodeInstance команды.
7. Retention в целых миллисекундах.
8. JSON массива `{segment, channelJson, eventId}` из Rust.

Payload и ключи строятся типизированным Rust-адаптером. Lua не кодирует JSON.
NodeInstance actor должен соответствовать lease store. Время команды не
используется: все события, closed_at_ms и deadline назначаются по Redis TIME.

## Проверки и изменения

До первой записи проверяются:

- Lease и deadline поколения, статус/generation соединения и generation index.
- Наличие connection state при непустых connection indexes.
- Точное совпадение набора каналов с connection channels.
- Ledger: формат highest serial и serial в order ZSET, нулевые scores,
  существование fingerprint/result у каждой сохраняемой операции.
- Владение каждым attachment, все удаляемые members и их обратные ссылки,
  counters/versions каждого канала и отсутствие переполнений.
- Суммарное число удаляемых members совпадает с SCARD connection members:
  ссылки на неизвестные каналы не должны молча удаляться.
- Тип outbox и возможность развернуть аргументы всех XADD до первого события.

Затем записываются все события, выполняется commit удаления каждого attachment,
удаляются connection channels/members, state получает status=closed и closed_at_ms.
Highest serial и сохранённые результаты операций остаются прежними.

Connection members читается одним SMEMBERS после согласования списка каналов.
Ссылки распределяются по каналам за один проход и передаются в общую подготовку
удаления: отсутствующие обратные ссылки и ссылки без member по-прежнему
отклоняются. Ссылка на неизвестный канал отклоняется до записей. Поэлементные
SREM из connection channels/members при полном закрытии пропускаются: оба SET
удаляются целиком через DEL. Обычный detach сохраняет поэлементное удаление.

Outbox использует общий формат `detach.v1`: полные неизменяемые данные события
на каждый канал. Event ID выводится в Rust из базового UUID команды и исходного
имени канала. События и результат возвращаются в порядке исходных имён каналов;
Leave внутри события сортируются по client ID общим Rust-декодером.

Как и для других Lua transitions, Redis не откатывает команды при runtime error.
Предсказуемые ошибки проверяются до записей; исчерпание ресурсов Redis требует
восстановления. Потеря ответа не доказывает отсутствие commit, события остаются
в outbox для публикации.

## Retention и индекс поколения

`deadline = Redis TIME + retention`. Один и тот же абсолютный срок через
PEXPIREAT получают connection state, order ZSET и каждый HASH операции.
Пустой ledger тоже оставляет закрытый connection state на время retention.
Retention=0 удаляет их сразу. Deadline должен помещаться в точные миллисекунды
`0..2^53-1`, переполнение отклоняется до записей.

Generation connections содержит только открытые соединения, включая соединения
без attachments с открытым ledger. Disconnect удаляет A.K из этого SET при
закрытии. Закрытые записи уже очищаются TTL и не требуют reaper; после их
истечения в generation index не остаётся висящей ссылки. Apply Presence и detach
проверяют этот контракт; закрытое состояние может существовать без записи в SET.

Повтор до истечения retention возвращает `{1, {}}` без записей: closed_at_ms,
TTL, counters и outbox не меняются. Сохраняемые Presence-операции продолжают
воспроизводиться через apply_presence, новые получают connectionClosed.
После истечения retention прежний ledger больше не восстанавливается.

## Ответ и оставшаяся работа

- `{1, transitions}` — завершение, включая пустой список для повтора.
- `{"channels", segments}` — подготовить актуальный набор и повторить вызов.
- `{0, code, message}` — ошибка хранилища, без domain ledger record.

`decode_disconnect` разбирает ответ, а каждый transition — общий decode_transition.
Для reaper готова очистка одного соединения; далее — поиск и обход истёкших
поколений, финализация registry; затем подключение traits/runtime,
outbox publisher/projector и поведенческие/нагрузочные проверки.
