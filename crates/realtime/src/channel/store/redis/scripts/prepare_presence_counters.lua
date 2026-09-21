-- Вызывать для успешно подготовленного непустого batch, до любых записей.
-- member_delta — итоговое изменение числа участников из prepare_presence_batch.
-- before прочитан через read_channel_state; revision проверена до batch.
-- Формат результата совпадает с prepare_attach_counters.
-- Перед этим фрагментом: read_channel_state.lua.
local function prepare_presence_counters(before, member_delta)
  if member_delta < -before.presence_members then
    return nil, {0, "corrupt_state", "Presence member count is below the removed count"}
  end
  if member_delta > MAX_COUNTER - before.presence_members then
    return nil, {0, "numeric_overflow", "Presence member count overflow"}
  end

  local members = before.presence_members + member_delta
  local occupancy_version = before.occupancy_version
  local changed, boundaries = {}, {}
  if member_delta ~= 0 then
    if occupancy_version == MAX_COUNTER then
      return nil, {0, "numeric_overflow", "occupancy version overflow"}
    end
    occupancy_version = occupancy_version + 1
    changed[1] = "presenceMembers"
    if before.presence_members == 0 or members == 0 then
      boundaries[1] = "presenceMembers"
    end
  end

  local metrics = {}
  for index = 1, 6 do
    local field = CHANNEL_STATE_FIELDS[index]
    metrics[field] = string.format("%.0f", before[field])
  end
  metrics.presence_members = string.format("%.0f", members)
  return {
    metrics = metrics,
    presence_revision = string.format("%.0f", before.presence_revision + 1),
    occupancy_version = string.format("%.0f", occupancy_version),
    presence_changed = true,
    changed_categories = changed,
    zero_boundary_categories = boundaries,
  }, nil
end
