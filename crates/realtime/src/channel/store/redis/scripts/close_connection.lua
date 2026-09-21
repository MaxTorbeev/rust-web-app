-- Общее закрытие соединения для disconnect и reaper. Вызывать после проверки leases.
-- KEYS[1..9] и ARGV[1..8]: docs/redis-disconnect.md; далее ключи каналов
-- начинаются после channel_keys_offset. Проверки всех каналов предшествуют записям.
-- indexed — результат SISMEMBER, если caller уже проверил его в этом Lua-вызове.
local function close_connection(now_ms, channel_keys_offset, indexed)
  local connection_exists = redis.call("TYPE", KEYS[3]).ok ~= "none"
  local connection = redis.call("HMGET", KEYS[3], "status", "generation", "highest_serial", "closed_at_ms")
  if connection_exists then
    if (connection[1] ~= "open" and connection[1] ~= "closed") or not connection[2] then
      return {0, "corrupt_state", "invalid connection status or generation"}
    end
    if connection[2] ~= ARGV[2] then
      return {0, "generation_mismatch", "connection belongs to another node generation"}
    end
  end
  local connection_reference = ARGV[3] .. "." .. ARGV[4]
  if indexed == nil then
    indexed = redis.call("SISMEMBER", KEYS[6], connection_reference) == 1
  end
  if indexed ~= (connection_exists and connection[1] == "open") then
    return {0, "corrupt_state", "connection state does not match its generation index"}
  end
  local actual = redis.call("SMEMBERS", KEYS[4])
  local member_count = redis.call("SCARD", KEYS[5])
  if not connection_exists and (#actual > 0 or member_count > 0) then
    return {0, "corrupt_state", "connection indexes exist without connection state"}
  end
  if connection[1] == "closed" then
    if #actual > 0 or member_count > 0 or not connection[4] then
      return {0, "corrupt_state", "invalid closed connection state"}
    end
    return {1, {}} -- Не продлеваем retention при повторе.
  end

  local channels = cjson.decode(ARGV[8])
  if #KEYS ~= channel_keys_offset + #channels * 3 then
    return {0, "invalid_request", "connection channel keys do not match arguments"}
  end
  if #actual ~= #channels then return {"channels", actual} end
  local remaining, indexed_members = {}, {}
  for _, channel in ipairs(actual) do remaining[channel] = true end
  for _, channel in ipairs(channels) do
    if not remaining[channel.segment] then return {"channels", actual} end
    remaining[channel.segment] = nil
    indexed_members[channel.segment] = {}
  end

  -- Один обход индекса соединения вместо SMEMBERS и фильтрации для каждого канала.
  for _, reference in ipairs(redis.call("SMEMBERS", KEYS[5])) do
    local channel = string.match(reference, "^(.-)%.")
    local indexed = indexed_members[channel]
    if not indexed then
      return {0, "corrupt_state", "connection member index references an unknown channel"}
    end
    indexed[reference] = true
  end

  local retention = tonumber(ARGV[7]) -- Целые миллисекунды из Duration в Rust.
  if not retention or retention < 0 or retention > MAX_COUNTER - now_ms then
    return {0, "invalid_request", "connection retention exceeds the supported timestamp range"}
  end
  local deadline = string.format("%.0f", now_ms + retention)
  local closed_at = string.format("%.0f", now_ms)
  local highest = connection[3]
  if highest and not valid_operation_serial(highest) then
    return {0, "corrupt_state", "invalid highest Presence operation serial"}
  end
  -- Проверяем HASH всех сохраняемых операций до удаления первого attachment.
  local order = redis.call("ZRANGE", KEYS[8], 0, -1, "WITHSCORES")
  local operations = {}
  for index = 1, #order, 2 do
    local serial = order[index]
    if not valid_operation_serial(serial) or order[index + 1] ~= "0" or not highest or serial > highest then
      return {0, "corrupt_state", "invalid Presence operation order"}
    end
    local key = KEYS[9] .. "." .. serial
    local record = redis.call("HMGET", key, "fingerprint", "result")
    if not record[1] or (record[2] ~= "committed" and record[2] ~= "rejected") then
      return {0, "corrupt_state", "invalid saved Presence operation"}
    end
    operations[#operations + 1] = key
  end

  local actor = {connectionId = ARGV[5], nodeInstance = cjson.decode(ARGV[6])}
  local plans, transitions, removed_count = {}, {}, 0
  for index, channel in ipairs(channels) do
    local offset = channel_keys_offset + (index - 1) * 3
    local state_key, attachments_key, members_key = KEYS[offset + 1], KEYS[offset + 2], KEYS[offset + 3]
    local previous, failure = read_attachment(attachments_key, ARGV[4], actor, connection_exists)
    if failure then return failure end
    if not previous then
      return {0, "corrupt_state", "connection channel index references a missing attachment"}
    end
    local prepared, failure = prepare_detach(
      state_key, members_key, KEYS[5], channel.segment, ARGV[4], actor, previous, true,
      indexed_members[channel.segment]
    )
    if failure then return failure end
    prepared.outbox = prepare_removal_outbox(
      {"format", "detach.v1", "node_instance_json", ARGV[6]},
      prepared.removed, prepared.counters, channel.channelJson, channel.eventId, now_ms
    )
    plans[index] = prepared
    transitions[index] = {"changed", prepared.outbox}
    removed_count = removed_count + #prepared.removed
  end
  if removed_count ~= member_count then
    return {0, "corrupt_state", "connection member index references an unknown channel"}
  end
  if #plans > 0 then
    local outbox_type = redis.call("TYPE", KEYS[7]).ok
    if outbox_type ~= "none" and outbox_type ~= "stream" then
      return {0, "corrupt_state", "outbox key must be a stream"}
    end
  end
  -- Проверяем разворачивание каждого списка аргументов до первого XADD.
  -- Иначе слишком большой следующий канал мог бы оставить предыдущие события.
  local function check_arguments(...) end
  for _, prepared in ipairs(plans) do
    check_arguments("XADD", KEYS[7], "*", unpack(prepared.outbox))
  end
  local response = {1, transitions}

  -- Commit: проверки всех каналов и ledger завершены.
  for _, prepared in ipairs(plans) do
    redis.call("XADD", KEYS[7], "*", unpack(prepared.outbox))
  end
  for index, prepared in ipairs(plans) do
    local offset = channel_keys_offset + (index - 1) * 3
    commit_detach(KEYS[offset + 1], KEYS[offset + 2], KEYS[offset + 3],
      nil, nil, channels[index].segment, ARGV[4], prepared)
  end
  redis.call("DEL", KEYS[4], KEYS[5])
  redis.call("HSET", KEYS[3], "status", "closed", "generation", ARGV[2], "closed_at_ms", closed_at)
  redis.call("SREM", KEYS[6], connection_reference)
  for _, key in ipairs(operations) do redis.call("PEXPIREAT", key, deadline) end
  redis.call("PEXPIREAT", KEYS[8], deadline)
  redis.call("PEXPIREAT", KEYS[3], deadline)
  return response
end
