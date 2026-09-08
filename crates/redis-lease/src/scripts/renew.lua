-- Продлевает lease, только если он принадлежит вызывающему в текущем периоде.
--
-- KEYS[1]: lease-ключ.
-- ARGV[1]: token периода `lease:<owner>:<fence>`.
-- ARGV[2]: новый TTL в целых положительных миллисекундах.
--
-- Ответы: 1 — продлён; 0 — lease истёк, освобождён или принадлежит другому
-- периоду (в том числе новому периоду того же владельца). Чужой lease никогда
-- не продлевается и не перезаписывается.

local key = KEYS[1]
local token_value = ARGV[1]
local ttl_ms = ARGV[2]

if redis.call("GET", key) ~= token_value then
  return 0
end

redis.call("PEXPIRE", key, ttl_ms)
return 1
