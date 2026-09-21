-- Добавляет к format/origin неизменяемые данные события attach/detach.
-- Payload удалённых участников подготовлен Rust; Lua JSON не кодирует.
local function prepare_removal_outbox(fields, removed, counters, channel_json, event_id, now_ms)
  local common = {
    "event_name", "realtime.presence_channel_changed", "schema_version", "1",
    "event_id", event_id, "occurred_at_ms", string.format("%.0f", now_ms),
    "channel_json", channel_json,
    "presence_revision", counters.presence_changed and counters.presence_revision or "",
    "occupancy_version", counters.occupancy_version,
    "removed_count", tostring(#removed),
  }
  for _, value in ipairs(common) do fields[#fields + 1] = value end
  append_occupancy_fields(fields, counters)
  for index, entry in ipairs(removed) do
    fields[#fields + 1] = "removed." .. index
    fields[#fields + 1] = entry.member_json
  end
  return fields
end
