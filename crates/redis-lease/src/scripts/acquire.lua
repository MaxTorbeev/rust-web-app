-- Атомарно захватывает lease или подтверждает, что он уже принадлежит вызывающему.
--
-- KEYS[1]: lease-ключ.
-- KEYS[2]: ключ монотонного fence-счётчика (`<lease-ключ>:fence`), без TTL.
--
-- ARGV[1]: значение владельца `lease:<owner>` — значение lease без fence.
-- ARGV[2]: TTL lease в целых положительных миллисекундах; Rust-код округляет
--          Duration вверх и проверяет диапазон до вызова.
--
-- В ключе хранится `lease:<owner>:<fence>` — идентичность периода владения, а
-- не владельца. Новый захват (ключа нет) увеличивает счётчик и записывает новый
-- fence; так token прошлого периода того же владельца перестаёт совпадать с
-- ключом, и его отложенные renew/release и проверки holds_lease не проходят.
--
-- Ответы:
--   {1, fence}        — lease у вызывающего. Повторный acquire тем же владельцем
--                       в непрерывном периоде только продлевает TTL и возвращает
--                       fence этого периода, поэтому потерянный ответ Redis не
--                       приводит ни к потере lease, ни к новому периоду.
--   {2, remaining_ms} — lease держит другой владелец; remaining_ms — остаток его TTL.
--
-- Между GET, INCR и SET/PEXPIRE другой клиент не может изменить ключи: скрипт
-- выполняется Redis атомарно, поэтому два конкурентных acquire не получат lease
-- одновременно, а fence нового периода всегда больше fence предыдущего.

local key = KEYS[1]
local fence_key = KEYS[2]
local owner_value = ARGV[1]
local ttl_ms = ARGV[2]

if not owner_value or string.sub(owner_value, 1, 6) ~= "lease:" or string.len(owner_value) <= 6 then
  return redis.error_reply("ERR invalid lease owner value")
end

local current = redis.call("GET", key)

if not current then
  local fence = redis.call("INCR", fence_key)
  -- SET с PX создаёт lease сразу с TTL: отдельные SET и PEXPIRE оставили бы
  -- окно, в котором lease мог сохраниться навсегда после сбоя владельца.
  redis.call("SET", key, owner_value .. ":" .. fence, "PX", ttl_ms)
  return {1, fence}
end

-- Fence — последний сегмент и только цифры; владелец может содержать `:`,
-- поэтому разбираем с конца. Значение другого вида этому крейту не принадлежит:
-- это повреждение или чужой ключ, а не «занято».
local current_owner, current_fence = string.match(current, "^(.*):(%d+)$")

if not current_owner then
  return redis.error_reply("ERR invalid lease value")
end

if current_owner == owner_value then
  redis.call("PEXPIRE", key, ttl_ms)
  return {1, tonumber(current_fence)}
end

local remaining_ms = redis.call("PTTL", key)

-- PTTL возвращает -1 для ключа без TTL и -2 для отсутствующего ключа. Lease без
-- TTL никогда не истечёт — это повреждённое состояние, а не «занято».
if remaining_ms < 0 then
  return redis.error_reply("ERR lease state has no valid TTL")
end

return {2, remaining_ms}
