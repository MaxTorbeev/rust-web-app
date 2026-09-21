-- Читает channel state и рассчитывает его новое состояние без записей.
-- old/new — таблицы из attachment_occupancy: какие счётчики канала старый/новый
-- attachment увеличивает на 1. Для этих счётчиков новое значение = текущее - old + new.
-- Таблицы не изменяются; для detach new содержит нулевые значения.
-- removed — план prepare_presence_removal.
-- state_exists — результат TYPE state_key ~= "none" в этом же вызове.
--
-- Успех: {metrics, presence_revision, occupancy_version, presence_changed,
--          changed_categories, zero_boundary_categories}, nil.
-- Счётчики и версии: 0..2^53-1, арифметика выполняется числами Lua.
-- Результат возвращается десятичными строками, категории — canonical camelCase.
-- Отказ: nil, {0, code, message}. Вызывать до любых записей transition.
-- Перед этим фрагментом: read_channel_state.lua.
local function prepare_attach_counters(state_key, old, new, removed, state_exists)
  local categories = {
    "connections",
    "publishers",
    "subscribers",
    "presenceConnections",
    "presenceSubscribers",
    "presenceMembers",
  }

  if not state_exists and (old.connections == 1 or #removed > 0) then
    return nil, {0, "corrupt_state", "attachment or members exist without channel state"}
  end
  local before, failure = read_channel_state(state_key, state_exists)
  if failure then return nil, failure end

  local after, changed, boundaries = {}, {}, {}
  for index = 1, 6 do
    local field = CHANNEL_STATE_FIELDS[index]
    local subtract, add
    if field == "presence_members" then
      subtract, add = #removed, 0
    else
      subtract, add = old[field], new[field]
    end
    if before[field] < subtract then
      return nil, {0, "corrupt_state", "channel counter is below the removed contribution: " .. field}
    end
    local value = before[field] - subtract
    if add > MAX_COUNTER - value then
      return nil, {0, "numeric_overflow", "channel counter overflow: " .. field}
    end
    value = value + add
    -- Не используем tostring: на больших значениях он может округлять цифры.
    after[field] = string.format("%.0f", value)
    if value ~= before[field] then
      changed[#changed + 1] = categories[index]
      if value == 0 or before[field] == 0 then
        boundaries[#boundaries + 1] = categories[index]
      end
    end
  end

  local presence_revision = before.presence_revision
  if #removed > 0 then
    if presence_revision == MAX_COUNTER then
      return nil, {0, "numeric_overflow", "presence revision overflow"}
    end
    presence_revision = presence_revision + 1
  end
  local occupancy_version = before.occupancy_version
  if #changed > 0 then
    if occupancy_version == MAX_COUNTER then
      return nil, {0, "numeric_overflow", "occupancy version overflow"}
    end
    occupancy_version = occupancy_version + 1
  end

  return {
    metrics = after,
    presence_revision = string.format("%.0f", presence_revision),
    occupancy_version = string.format("%.0f", occupancy_version),
    presence_changed = #removed > 0,
    changed_categories = changed,
    zero_boundary_categories = boundaries,
  }, nil
end
