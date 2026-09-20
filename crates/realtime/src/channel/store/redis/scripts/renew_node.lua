-- Продлевает node lease и обновляет deadline его поколения.
-- Перед этим скриптом должно быть определение функции renew_lease.
--
-- KEYS[1]: ключ node lease.
-- KEYS[2]: ZSET с deadlines поколений.
--
-- ARGV[1]: полный token lease:<generation>:<fence>.
-- ARGV[2]: TTL в целых положительных миллисекундах.
-- ARGV[3]: идентификатор поколения.
--
-- Ответ: 1 — lease и deadline продлены; 0 — владение потеряно, записей нет.

if #KEYS ~= 2 or #ARGV ~= 3 then
  return redis.error_reply("ERR invalid node renewal arguments")
end

local lease_key = KEYS[1]
local deadlines_key = KEYS[2]
local token_value = ARGV[1]
local ttl_value = ARGV[2]
local generation = ARGV[3]

local token_owner = string.match(token_value, "^(lease:.+):[1-9]%d*$")
if generation == "" or token_owner ~= "lease:" .. generation then
  return redis.error_reply("ERR node lease token does not match generation")
end

if not string.match(ttl_value, "^[1-9]%d*$") then
  return redis.error_reply("ERR invalid node lease TTL")
end

local max_exact_integer = 9007199254740991
local ttl_ms = tonumber(ttl_value)
if not ttl_ms or ttl_ms > max_exact_integer then
  return redis.error_reply("ERR node lease TTL exceeds exact integer range")
end

-- Проверяем тип до продления: ошибка Lua не откатывает предыдущие записи.
local deadlines_type = redis.call("TYPE", deadlines_key).ok
if deadlines_type ~= "none" and deadlines_type ~= "zset" then
  return redis.error_reply("ERR node generation deadlines key must be a zset")
end

local time = redis.call("TIME")
local now_ms = tonumber(time[1]) * 1000
  + math.floor(tonumber(time[2]) / 1000)

if ttl_ms > max_exact_integer - now_ms then
  return redis.error_reply("ERR node lease deadline exceeds exact integer range")
end

local deadline_value = string.format("%.0f", now_ms + ttl_ms)

-- Истёкший или старый token не продлевает lease и не обновляет индекс.
if renew_lease(lease_key, token_value, ttl_value) ~= 1 then
  return 0
end

redis.call("PEXPIREAT", lease_key, deadline_value)
redis.call("ZADD", deadlines_key, deadline_value, generation)
return 1
