# Redis: контракт snapshot

`RedisChannelStore::snapshot` читает состояние канала через `scripts::SNAPSHOT`
одним Lua-вызовом. Метод подключён через `PresenceStore` к runtime и используется
для initial SYNC и восстановления после пропуска revision.

## Входы и ответ

Все ключи строятся через `RedisKeys` для переданного `ChannelKey`:

1. `channel_state(channel)`.
2. `channel_members(channel)`.
3. `channel_attachments(channel)`.
4. `channel_shards(channel)`.

Аргументов нет. Ответ совпадает с полем snapshot операции attach:
`{members, presence_revision, occupancy_version, metrics}`. Members — массив
троек `{payload_json, revision, updated_at}`, metrics — шесть пар field/value в порядке `decode_metrics`.
Версии и счётчики проверяются общим `read_channel_state` и возвращаются
десятичными строками без потери точности.
Rust использует общий `decode_snapshot` и сортирует участников по
`(connection_id, client_id)`.

## Поведение

- Чтение не меняет состояние, версии или outbox и не требует node lease.
- Отсутствующий канал возвращает пустой список, нулевые версии и счётчики.
  Members, attachments или shards без channel state считаются повреждением.
- Существующий state должен содержать все восемь числовых полей в диапазоне
  `0..2^53-1`; число members должно совпадать с `presence_members`.
- Ошибки Redis, повреждённые counters и невалидный JSON участников становятся
  `ChannelStateStoreError::Internal`, без подмены результата пустым снимком.
- Snapshot возвращает сохранённое состояние. Удаление участников истёкших
  поколений относится к reaper и не выполняется при чтении.
