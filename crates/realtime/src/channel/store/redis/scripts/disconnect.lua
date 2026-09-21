-- Сначала проверяет весь набор каналов, затем удаляет attachments и закрывает ledger.
-- JSON каналов и event ID готовит Rust. KEYS/ARGV: docs/redis-disconnect.md.
if #KEYS < 9 or #ARGV ~= 8 then
  return {0, "invalid_request", "disconnect requires 9 base keys and 8 arguments"}
end
local now_ms, failure = check_node_lease(KEYS[1], KEYS[2], ARGV[1], ARGV[2])
if failure then return failure end
return close_connection(now_ms, 9)
