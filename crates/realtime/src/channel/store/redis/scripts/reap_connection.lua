-- Закрывает одно соединение истёкшего поколения. KEYS/ARGV: docs/redis-reaper.md.
if #KEYS < 11 or #ARGV ~= 10 then
  return {0, "invalid_request", "reap_connection requires 11 base keys and 10 arguments"}
end
local now_ms, failure = check_reap_generation(KEYS[1], KEYS[2], KEYS[10], KEYS[11],
  ARGV[1], ARGV[9], ARGV[10], ARGV[2])
if failure then return failure end

-- Уже очищенное соединение не создаёт повторный tombstone после истечения retention.
if redis.call("SISMEMBER", KEYS[6], ARGV[3] .. "." .. ARGV[4]) == 0 then
  return {1, {}}
end
return close_connection(now_ms, 11, true)
