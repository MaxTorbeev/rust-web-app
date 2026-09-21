-- Presence batch, ledger и outbox фиксируются одним Lua-вызовом.
-- Порядок KEYS/ARGV: docs/redis-apply-presence.md. JSON подготовлен Rust.
if #KEYS ~= 12 or #ARGV ~= 13 then
  return {0, "invalid_request", "apply_presence requires 12 keys and 13 arguments"}
end

local now_ms, failure = check_node_lease(KEYS[1], KEYS[2], ARGV[1], ARGV[2])
if failure then return failure end
local connection_exists = redis.call("TYPE", KEYS[6]).ok ~= "none"
local connection = redis.call("HMGET", KEYS[6], "status", "generation", "highest_serial")
local highest = connection[3]
if connection_exists then
  if (connection[1] ~= "open" and connection[1] ~= "closed") or not connection[2] then
    return {0, "corrupt_state", "invalid connection status or generation"}
  end
  if connection[2] ~= ARGV[2] then
    return {0, "generation_mismatch", "connection belongs to another node generation"}
  end
end
if highest and not valid_operation_serial(highest) then
  return {0, "corrupt_state", "invalid highest Presence operation serial"}
end
local connection_reference = ARGV[3] .. "." .. ARGV[5]
if (redis.call("SISMEMBER", KEYS[9], connection_reference) == 1) ~= (connection_exists and connection[1] == "open") then
  return {0, "corrupt_state", "connection state does not match its generation index"}
end
local operation_count = redis.call("ZCARD", KEYS[12])
if operation_count > 0 and not highest then
  return {0, "corrupt_state", "operation index exists without highest serial"}
end
local known, failure = lookup_presence_operation(
  KEYS[11], KEYS[12], ARGV[10], ARGV[11], highest, connection[1] == "closed"
)
if failure then return failure end
if known then return known end

if not connection_exists
  and (redis.call("SCARD", KEYS[7]) ~= 0 or redis.call("SCARD", KEYS[8]) ~= 0) then
  return {0, "corrupt_state", "connection indexes exist without connection state"}
end
local indexed_channel = redis.call("SISMEMBER", KEYS[7], ARGV[4]) == 1

-- Возвращает готовые записи либо доменный/инфраструктурный отказ, без записей.
local function prepare_mutation()
  local state_type = redis.call("TYPE", KEYS[3]).ok
  if state_type == "none" then
    if indexed_channel or redis.call("HLEN", KEYS[4]) ~= 0 or redis.call("HLEN", KEYS[5]) ~= 0 then
      return nil, {0, "corrupt_state", "channel data exists without channel state"}
    end
    return nil, {"rejected", "notAttached"}
  end
  if state_type ~= "hash" then
    return nil, {0, "corrupt_state", "channel state must be a hash"}
  end
  local items = cjson.decode(ARGV[9])
  if #items == 0 then
    return nil, {0, "invalid_request", "presence batch must contain at least one item"}
  end
  local actor = {connectionId = ARGV[7], nodeInstance = cjson.decode(ARGV[8])}
  local attachment, failure = check_presence_attachment(KEYS[4], ARGV[5], actor, connection_exists)
  if failure and failure[1] == 0 then return nil, failure end
  if indexed_channel ~= (attachment ~= false) then
    return nil, {0, "corrupt_state", "attachment does not match its connection channel index"}
  end
  if failure then return nil, failure end

  local before, failure = read_channel_state(KEYS[3], true)
  if failure then return nil, failure end
  -- Как в memory store: переполнение revision проверяется до элементов batch.
  if before.presence_revision == MAX_COUNTER then
    return nil, {0, "numeric_overflow", "presence revision overflow"}
  end
  local plan, failure = prepare_presence_batch(KEYS[5], KEYS[8], ARGV[4], ARGV[5], attachment, items)
  if failure then return nil, failure end
  local counters, failure = prepare_presence_counters(before, plan.member_delta)
  if failure then return nil, failure end
  local prepared = prepare_presence_result(plan, items, counters, ARGV[6], ARGV[8], ARGV[12], now_ms)
  prepared.counters = counters
  return prepared, nil
end

local prepared, outcome = prepare_mutation()
if outcome and outcome[1] == 0 then return outcome end
if prepared then
  outcome = prepared.outcome
  local outbox_type = redis.call("TYPE", KEYS[10]).ok
  if outbox_type ~= "none" and outbox_type ~= "stream" then
    return {0, "corrupt_state", "outbox key must be a stream"}
  end
end
local operation = prepare_presence_operation(ARGV[11], outcome)
local capacity = tonumber(ARGV[13]) -- Rust передаёт usize, не меньше 1.
local excess = operation_count + 1 - capacity
local evicted = {}
if excess > 0 then
  evicted = redis.call("ZRANGE", KEYS[12], 0, excess - 1)
  for _, serial in ipairs(evicted) do
    if not valid_operation_serial(serial) then
      return {0, "corrupt_state", "invalid Presence operation order serial"}
    end
  end
  -- Новая операция тоже может выпасть из окна, если capacity уменьшили.
  if ARGV[10] < evicted[#evicted] then
    evicted[#evicted] = ARGV[10]
  end
end
local operation_prefix = string.sub(KEYS[11], 1, #KEYS[11] - 20)
local next_highest = highest and highest > ARGV[10] and highest or ARGV[10]

-- Commit: все проверки и подготовка закончены. XADD первым, чтобы ошибка
-- unpack большого outbox не оставляла изменённые state или ledger.
if prepared then
  redis.call("XADD", KEYS[10], "*", unpack(prepared.outbox))
  for _, entry in ipairs(prepared.members) do
    local field = entry.member_field
    if entry.member_json then
      redis.call("HSET", KEYS[5], field, entry.member_json,
        field .. ":revision", prepared.presence_revision, field .. ":updated_at", prepared.updated_at)
      redis.call("SADD", KEYS[8], entry.connection_member)
    else
      redis.call("HDEL", KEYS[5], field, field .. ":revision", field .. ":updated_at")
      redis.call("SREM", KEYS[8], entry.connection_member)
    end
  end
  redis.call("HSET", KEYS[3], "presence_members", prepared.counters.metrics.presence_members,
    "presence_revision", prepared.presence_revision, "occupancy_version", prepared.counters.occupancy_version)
end
if not connection_exists then
  redis.call("HSET", KEYS[6], "status", "open", "generation", ARGV[2])
  redis.call("SADD", KEYS[9], connection_reference)
end
-- По паре полей: размер ledger-записи не ограничен Lua unpack.
for index = 1, #operation, 2 do
  redis.call("HSET", KEYS[11], operation[index], operation[index + 1])
end
redis.call("ZADD", KEYS[12], 0, ARGV[10])
redis.call("HSET", KEYS[6], "highest_serial", next_highest)
for _, serial in ipairs(evicted) do
  redis.call("DEL", operation_prefix .. serial)
  redis.call("ZREM", KEYS[12], serial)
end
return outcome
