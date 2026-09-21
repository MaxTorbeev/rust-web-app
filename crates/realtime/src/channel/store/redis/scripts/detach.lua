-- Удаляет attachment и его участников одним вызовом. Ledger остаётся открытым.
-- KEYS/ARGV: docs/redis-detach.md. Ответ: transition либо {0, code, message}.
if #KEYS ~= 10 or #ARGV ~= 9 then
  return {0, "invalid_request", "detach requires 10 keys and 9 arguments"}
end
local now_ms, failure = check_node_lease(KEYS[1], KEYS[2], ARGV[1], ARGV[2])
if failure then return failure end
local connection_exists = redis.call("TYPE", KEYS[6]).ok ~= "none"
local connection = redis.call("HMGET", KEYS[6], "status", "generation")
if connection_exists then
  if (connection[1] ~= "open" and connection[1] ~= "closed") or not connection[2] then
    return {0, "corrupt_state", "invalid connection status or generation"}
  end
  if connection[2] ~= ARGV[2] then
    return {0, "generation_mismatch", "connection belongs to another node generation"}
  end
end
local actor = {connectionId = ARGV[7], nodeInstance = cjson.decode(ARGV[8])}
local previous, failure = read_attachment(KEYS[4], ARGV[5], actor, connection_exists)
if failure then return failure end
if (redis.call("SISMEMBER", KEYS[7], ARGV[4]) == 1) ~= (previous ~= false) then
  return {0, "corrupt_state", "attachment does not match its connection channel index"}
end
if (redis.call("SISMEMBER", KEYS[9], ARGV[3] .. "." .. ARGV[5]) == 1) ~= (connection_exists and connection[1] == "open") then
  return {0, "corrupt_state", "connection state does not match its generation index"}
end
if not connection_exists
  and (redis.call("SCARD", KEYS[7]) ~= 0 or redis.call("SCARD", KEYS[8]) ~= 0) then
  return {0, "corrupt_state", "connection indexes exist without connection state"}
end
local state_exists = redis.call("TYPE", KEYS[3]).ok ~= "none"
if not state_exists and (redis.call("HLEN", KEYS[4]) ~= 0 or redis.call("HLEN", KEYS[5]) ~= 0) then
  return {0, "corrupt_state", "channel data exists without channel state"}
end
if not previous then
  local state, failure = read_channel_state(KEYS[3], state_exists)
  if failure then return failure end
  return {"unchanged", string.format("%.0f", state.occupancy_version)}
end
if connection[1] == "closed" then
  return {0, "corrupt_state", "closed connection still has an attachment"}
end

local prepared, failure = prepare_detach(
  KEYS[3], KEYS[5], KEYS[8], ARGV[4], ARGV[5], actor, previous, state_exists
)
if failure then return failure end
local outbox = prepare_removal_outbox(
  {"format", "detach.v1", "node_instance_json", ARGV[8]},
  prepared.removed, prepared.counters, ARGV[6], ARGV[9], now_ms
)
local outbox_type = redis.call("TYPE", KEYS[10]).ok
if outbox_type ~= "none" and outbox_type ~= "stream" then
  return {0, "corrupt_state", "outbox key must be a stream"}
end
local response = {"changed", outbox}

-- Commit. XADD первым: ошибка unpack не оставит частично удалённое состояние.
redis.call("XADD", KEYS[10], "*", unpack(outbox))
commit_detach(KEYS[3], KEYS[4], KEYS[5], KEYS[7], KEYS[8], ARGV[4], ARGV[5], prepared)
return response
