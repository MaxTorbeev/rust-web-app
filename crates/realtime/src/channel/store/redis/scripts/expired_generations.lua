-- Кандидаты для reaper без записей и проверки lease.
-- KEYS: generation deadlines, generations. ARGV: limit (u32 из Rust).
-- Ответ: массив {generation, исходный JSON NodeInstance}; JSON разбирает Rust.
if #KEYS ~= 2 or #ARGV ~= 1 then
  return redis.error_reply("expired_generations requires 2 keys and 1 argument")
end
local time = redis.call("TIME")
local now_ms = tonumber(time[1]) * 1000 + math.floor(tonumber(time[2]) / 1000)
local generations = redis.call("ZRANGEBYSCORE", KEYS[1], "-inf",
  string.format("%.0f", now_ms), "LIMIT", 0, ARGV[1])
local entries = {}
for index, generation in ipairs(generations) do
  local metadata = redis.call("HGET", KEYS[2], generation)
  if not metadata then
    return redis.error_reply("expired generation metadata is missing")
  end
  entries[index] = {generation, metadata}
end
return entries
