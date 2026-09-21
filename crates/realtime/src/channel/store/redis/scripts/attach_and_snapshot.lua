-- Индивидуальный attach и snapshot после изменения в одном Redis-вызове.
-- Перед этим фрагментом: holds_lease, check_node_lease, check_attach_state,
-- attachment_occupancy, prepare_presence_removal, prepare_attach_counters,
-- prepare_attach_result. Порядок KEYS/ARGV: docs/redis-attach-and-snapshot.md.
-- Успех: {1, snapshot, transition}; отказ до записей: {0, code, message}.

if #KEYS ~= 10 or #ARGV ~= 8 then
  return {0, "invalid_request", "attach requires 10 keys and 8 arguments"}
end

-- Rust проверяет команду и строит из неё JSON, сегменты и ключи.
-- Lua читает только attachment для сравнения с сохранённым состоянием.
-- Исходные JSON сохраняются без перекодирования через cjson.
local attachment = cjson.decode(ARGV[7])
local instance = attachment.nodeInstance
if ARGV[2] ~= member_segment(instance.nodeId) .. "." .. instance.bootGeneration then
  return {0, "invalid_request", "attachment does not match the lease generation"}
end

local now_ms, rejection = check_node_lease(KEYS[1], KEYS[2], ARGV[1], ARGV[2])
if rejection then return rejection end
-- TYPE нужен здесь для различения отсутствующего state и HASH без нужных полей.
-- Неверные типы остальных читаемых ключей отклонят Redis-команды до commit.
local connection_exists = redis.call("TYPE", KEYS[6]).ok ~= "none"
local state_exists = redis.call("TYPE", KEYS[3]).ok ~= "none"
local previous, rejection = check_attach_state(
  KEYS[6], KEYS[4], ARGV[5], ARGV[2], attachment, connection_exists
)
if rejection then return rejection end

local connection_reference = ARGV[3] .. "." .. ARGV[5]
local indexed_channel = redis.call("SISMEMBER", KEYS[7], ARGV[4]) == 1
if indexed_channel ~= (previous ~= false) then
  return {0, "corrupt_state", "attachment does not match its connection channel index"}
end
local indexed_connection = redis.call("SISMEMBER", KEYS[9], connection_reference) == 1
if indexed_connection ~= connection_exists then
  return {0, "corrupt_state", "connection state does not match its generation index"}
end
if not connection_exists
  and (redis.call("SCARD", KEYS[7]) ~= 0 or redis.call("SCARD", KEYS[8]) ~= 0) then
  return {0, "corrupt_state", "connection indexes exist without connection state"}
end
if not state_exists and redis.call("HLEN", KEYS[4]) ~= 0 then
  return {0, "corrupt_state", "attachments exist without channel state"}
end

-- Единственное чтение members и один расчёт для каждого attachment.
local members, rejection = read_presence_members(KEYS[5])
if rejection then return rejection end
local old = attachment_occupancy(previous)
local new = attachment_occupancy(attachment)
local removed = {}
if old.presence_connections == 1 and new.presence_connections == 0 then
  removed, rejection = prepare_presence_removal(members, KEYS[8], ARGV[4], ARGV[5], attachment)
  if rejection then return rejection end
end
local counters, rejection = prepare_attach_counters(KEYS[3], old, new, removed, state_exists)
if rejection then return rejection end
local prepared, rejection = prepare_attach_result(members, removed, counters, ARGV[6], ARGV[7], ARGV[8], now_ms)
if rejection then return rejection end

-- Outbox не читается перед записью, поэтому его тип проверяем отдельно.
if prepared.outbox then
  local outbox_type = redis.call("TYPE", KEYS[10]).ok
  if outbox_type ~= "none" and outbox_type ~= "stream" then
    return {0, "corrupt_state", "outbox key must be a stream"}
  end
end

local state_values = {}
for _, value in ipairs(prepared.snapshot[4]) do state_values[#state_values + 1] = value end
state_values[#state_values + 1] = "presence_revision"
state_values[#state_values + 1] = counters.presence_revision
state_values[#state_values + 1] = "occupancy_version"
state_values[#state_values + 1] = counters.occupancy_version
local response = {1, prepared.snapshot, prepared.transition}

-- Commit. Все проверки и подготовка ответа завершены до первой записи.
-- XADD первым: ошибка unpack большого списка полей не оставит частичный state.
if prepared.outbox then
  redis.call("XADD", KEYS[10], "*", unpack(prepared.outbox))
end
-- Существующие поля connection state (включая highest_serial) сохраняются.
if not connection_exists then
  redis.call("HSET", KEYS[6], "generation", ARGV[2], "status", "open")
end
redis.call("HSET", KEYS[4], ARGV[5], ARGV[7])
redis.call("SADD", KEYS[7], ARGV[4])
redis.call("SADD", KEYS[9], connection_reference)
for _, entry in ipairs(removed) do
  -- По одному member: размер списка не ограничивается пределом Lua unpack.
  redis.call("HDEL", KEYS[5], entry.member_field, entry.member_field .. ":revision", entry.member_field .. ":updated_at")
  redis.call("SREM", KEYS[8], entry.connection_member)
end
redis.call("HSET", KEYS[3], unpack(state_values))
return response
