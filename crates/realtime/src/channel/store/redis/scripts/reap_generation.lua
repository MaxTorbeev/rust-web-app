-- KEYS: reaper lease, deadlines, cleanup lease, target lease, connections, shards, generations.
-- ARGV: reaper token, reaper generation, cleanup token, target generation, batch size.
if #KEYS ~= 7 or #ARGV ~= 5 then return redis.error_reply("invalid reap_generation arguments") end
local _, failure = check_reap_generation(KEYS[1], KEYS[2], KEYS[3], KEYS[4],
  ARGV[1], ARGV[2], ARGV[3], ARGV[4])
if failure then return failure end
local count = tonumber(ARGV[5])
if not count or count < 1 then return redis.error_reply("cleanup batch must be positive") end
local connections = redis.call("SRANDMEMBER", KEYS[5], count)
if #connections > 0 then return {1, connections, 0} end
if redis.call("SCARD", KEYS[6]) > 0 then
  return {0, "corrupt_state", "generation has aggregated shards requiring shard cleanup"}
end
-- Проверка типа registry до первой записи; fence counters и новый node lease не трогаем.
redis.call("HGET", KEYS[7], ARGV[4])
redis.call("HDEL", KEYS[7], ARGV[4])
redis.call("ZREM", KEYS[2], ARGV[4])
redis.call("DEL", KEYS[5], KEYS[6])
return {1, {}, 1}
