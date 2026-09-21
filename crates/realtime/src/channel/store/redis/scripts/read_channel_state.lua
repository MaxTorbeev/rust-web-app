-- Общий формат channel state. Значения проверяются до арифметики и записей.
local MAX_COUNTER = 9007199254740991
local CHANNEL_STATE_FIELDS = {
  "connections", "publishers", "subscribers", "presence_connections",
  "presence_subscribers", "presence_members", "presence_revision", "occupancy_version",
}

-- state_exists получен вызывающим скриптом; false разрешает новый пустой канал.
-- Успех: таблица числовых значений, nil. Отказ: nil, {0, code, message}.
local function read_channel_state(state_key, state_exists)
  local values = redis.call("HMGET", state_key, unpack(CHANNEL_STATE_FIELDS))
  local state = {}
  for index, field in ipairs(CHANNEL_STATE_FIELDS) do
    local value = values[index]
    if not state_exists then value = "0" end
    if type(value) ~= "string" or (value ~= "0" and not string.match(value, "^[1-9]%d*$")) then
      return nil, {0, "corrupt_state", "invalid or missing channel counter field: " .. field}
    end
    local number = tonumber(value)
    if not number or number > MAX_COUNTER then
      return nil, {0, "corrupt_state", "channel counter exceeds the supported range: " .. field}
    end
    state[field] = number
  end
  return state, nil
end
