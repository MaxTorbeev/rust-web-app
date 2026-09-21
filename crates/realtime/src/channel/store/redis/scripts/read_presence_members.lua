-- HASH: K.U -> payload из Rust; K.U:revision / K.U:updated_at -> числа строками.
-- Возвращает field/{payload, revision, timestamp} для attach и snapshot.
local function read_presence_members(key)
  local values = redis.call("HGETALL", key)
  local fields, members = {}, {}
  for index = 1, #values, 2 do fields[values[index]] = values[index + 1] end
  for index = 1, #values, 2 do
    local field = values[index]
    if not string.find(field, ":", 1, true) then
      local revision, timestamp = fields[field .. ":revision"], fields[field .. ":updated_at"]
      if not revision or not timestamp then
        return nil, {0, "corrupt_state", "missing Presence member revision or timestamp"}
      end
      members[#members + 1] = field
      members[#members + 1] = {values[index + 1], revision, timestamp}
    end
  end
  if #values ~= #members * 3 then
    return nil, {0, "corrupt_state", "orphan Presence member metadata"}
  end
  return members, nil
end
