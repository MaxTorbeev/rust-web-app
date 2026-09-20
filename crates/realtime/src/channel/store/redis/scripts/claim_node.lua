-- Захватывает node lease и регистрирует поколение ноды.
-- Перед этим скриптом должно быть определение функции acquire_lease.
--
-- KEYS[1]: ключ node lease.
-- KEYS[2]: ключ fence-счётчика.
-- KEYS[3]: HASH с метаданными поколений.
-- KEYS[4]: ZSET с deadlines поколений.
--
-- ARGV[1]: владелец lease в формате lease:<generation>.
-- ARGV[2]: TTL в целых положительных миллисекундах.
-- ARGV[3]: идентификатор поколения.
-- ARGV[4]: JSON с метаданными NodeInstance.
--
-- Ответ:
--   {1, fence}        — lease захвачен, поколение зарегистрировано.
--   {2, remaining_ms} — lease занят другим владельцем.
-- Fence возвращается десятичной строкой.

if #KEYS ~= 4 or #ARGV ~= 4 then
  return redis.error_reply("ERR invalid node claim arguments")
end

local lease_key = KEYS[1]
local fence_key = KEYS[2]
local generations_key = KEYS[3]
local deadlines_key = KEYS[4]

local owner_value = ARGV[1]
local ttl_value = ARGV[2]
local generation = ARGV[3]
local metadata = ARGV[4]

if generation == "" then
  return redis.error_reply("ERR empty node generation")
end

if owner_value ~= "lease:" .. generation then
  return redis.error_reply("ERR node lease owner does not match generation")
end

if metadata == "" then
  return redis.error_reply("ERR empty node generation metadata")
end

-- TTL должен быть положительным целым числом в десятичной записи.
if not string.match(ttl_value, "^[1-9]%d*$") then
  return redis.error_reply("ERR invalid node lease TTL")
end

-- Deadline будет числом в Lua и score в ZSET.
-- Ограничиваем вычисления диапазоном точных целых.
local max_exact_integer = 9007199254740991
local ttl_ms = tonumber(ttl_value)

if not ttl_ms or ttl_ms > max_exact_integer then
  return redis.error_reply("ERR node lease TTL exceeds exact integer range")
end

-- Отсутствующие ключи допустимы: HSET и ZADD создадут их при записи.
local generations_type = redis.call("TYPE", generations_key).ok

if generations_type ~= "none" and generations_type ~= "hash" then
  return redis.error_reply("ERR node generations key must be a hash")
end

local deadlines_type = redis.call("TYPE", deadlines_key).ok

if deadlines_type ~= "none" and deadlines_type ~= "zset" then
  return redis.error_reply("ERR node generation deadlines key must be a zset")
end

-- Повторный claim того же запуска должен передавать прежние метаданные.
local stored_metadata = redis.call("HGET", generations_key, generation)

if stored_metadata and stored_metadata ~= metadata then
  return redis.error_reply("ERR node generation metadata mismatch")
end

-- Redis TIME возвращает секунды и микросекунды.
local time = redis.call("TIME")
local now_ms = tonumber(time[1]) * 1000
  + math.floor(tonumber(time[2]) / 1000)

-- Проверяем диапазон до сложения, чтобы не потерять точность.
if ttl_ms > max_exact_integer - now_ms then
  return redis.error_reply("ERR node lease deadline exceeds exact integer range")
end

local deadline_ms = now_ms + ttl_ms
local deadline_value = string.format("%.0f", deadline_ms)

-- Захват может вернуть успех, занятость или ошибку.
local acquired = acquire_lease(
  lease_key,
  fence_key,
  owner_value,
  ttl_value
)

if acquired[1] ~= 1 then
  return acquired
end

-- Lease и generation index получают одинаковый абсолютный deadline.
redis.call("PEXPIREAT", lease_key, deadline_value)
redis.call("HSET", generations_key, generation, metadata)
redis.call("ZADD", deadlines_key, deadline_value, generation)

return acquired
