-- Освобождает lease, только если он принадлежит вызывающему в текущем периоде.
--
-- KEYS[1]: lease-ключ.
-- ARGV[1]: token периода `lease:<owner>:<fence>`.
--
-- Ответы: 1 — удалён; 0 — lease уже не принадлежал этому периоду, ничего не
-- тронуто. Сверяется token целиком, а не владелец: отложенный повтор release из
-- прошлого периода не удалит новый lease того же владельца.
-- Fence-счётчик не удаляется: он должен пережить release, чтобы следующий
-- период получил fence строго больше.

local key = KEYS[1]
local token_value = ARGV[1]

if redis.call("GET", key) ~= token_value then
  return 0
end

redis.call("DEL", key)
return 1
