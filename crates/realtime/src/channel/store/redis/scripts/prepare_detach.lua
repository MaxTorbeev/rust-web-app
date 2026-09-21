-- Готовит удаление members и counters одного канала без записей.
-- Lease, владелец attachment и индексы проверены вызывающим transition.
-- indexed_members передаётся при закрытии соединения из общего чтения его SET.
local function prepare_detach(state_key, members_key, connection_members_key,
  channel_segment, connection_segment, actor, previous, state_exists, indexed_members)
  local members, failure = read_presence_members(members_key)
  if failure then return nil, failure end
  local removed, failure = prepare_presence_removal(
    members, connection_members_key, channel_segment, connection_segment, actor, indexed_members
  )
  if failure then return nil, failure end
  local counters, failure = prepare_attach_counters(
    state_key, attachment_occupancy(previous), attachment_occupancy(false), removed, state_exists
  )
  if failure then return nil, failure end
  if string.format("%.0f", #members / 2 - #removed) ~= counters.metrics.presence_members then
    return nil, {0, "corrupt_state", "Presence member count does not match channel state"}
  end
  local state_values = {}
  for index = 1, 6 do
    local field = CHANNEL_STATE_FIELDS[index]
    state_values[#state_values + 1] = field
    state_values[#state_values + 1] = counters.metrics[field]
  end
  state_values[#state_values + 1] = "presence_revision"
  state_values[#state_values + 1] = counters.presence_revision
  state_values[#state_values + 1] = "occupancy_version"
  state_values[#state_values + 1] = counters.occupancy_version
  return {removed = removed, counters = counters, state_values = state_values}, nil
end

-- Вызывать только после подготовки всех затронутых каналов и записи outbox.
-- Ключи connection indexes можно не передавать, если caller удалит их целиком.
local function commit_detach(state_key, attachments_key, members_key,
  connection_channels_key, connection_members_key, channel_segment, connection_segment, prepared)
  redis.call("HDEL", attachments_key, connection_segment)
  if connection_channels_key then redis.call("SREM", connection_channels_key, channel_segment) end
  for _, entry in ipairs(prepared.removed) do
    local field = entry.member_field
    redis.call("HDEL", members_key, field, field .. ":revision", field .. ":updated_at")
    if connection_members_key then redis.call("SREM", connection_members_key, entry.connection_member) end
  end
  redis.call("HSET", state_key, unpack(prepared.state_values))
end
