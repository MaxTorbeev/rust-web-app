-- Читает согласованный snapshot без записей и проверки lease.
-- KEYS: channel state, members, attachments, shards. ARGV отсутствуют.
-- Ответ: {members {payload, revision, timestamp}, presence_revision, occupancy_version, metrics field/value}.
-- Повреждённое состояние возвращается как Redis error; JSON разбирает Rust.
-- Перед этим фрагментом: read_channel_state.lua и read_presence_members.lua.
if #KEYS ~= 4 or #ARGV ~= 0 then
  return redis.error_reply("snapshot requires 4 keys and no arguments")
end

local state_exists = redis.call("TYPE", KEYS[1]).ok ~= "none"
local entries, failure = read_presence_members(KEYS[2])
if failure then return redis.error_reply(failure[3]) end
local members = {}
for index = 2, #entries, 2 do members[#members + 1] = entries[index] end

if not state_exists then
  if #members > 0 or redis.call("EXISTS", KEYS[3], KEYS[4]) > 0 then
    return redis.error_reply("channel data exists without channel state")
  end
end

local state, failure = read_channel_state(KEYS[1], state_exists)
if failure then return redis.error_reply(failure[3]) end
local metrics = {}
for index = 1, 6 do
  local field = CHANNEL_STATE_FIELDS[index]
  metrics[#metrics + 1] = field
  metrics[#metrics + 1] = string.format("%.0f", state[field])
end

if #members ~= state.presence_members then
  return redis.error_reply("Presence member count does not match channel state")
end

return {members, string.format("%.0f", state.presence_revision), string.format("%.0f", state.occupancy_version), metrics}
