-- Материализует успешный batch до любых записей, без Redis I/O и JSON-кодирования.
-- memberJson подготовлен Rust; revision и время Redis хранятся отдельно.
local function prepare_presence_result(plan, items, counters, channel_json, node_instance_json, event_id, now_ms)
  local timestamp = string.format("%.0f", now_ms)
  local outbox = {
    "format", "presence.v2",
    "event_name", "realtime.presence_channel_changed", "schema_version", "1",
    "event_id", event_id, "occurred_at_ms", timestamp,
    "channel_json", channel_json, "node_instance_json", node_instance_json,
    "presence_revision", counters.presence_revision,
    "occupancy_version", counters.occupancy_version,
    "change_count", tostring(#plan.changes),
  }
  append_occupancy_fields(outbox, counters)
  for index, change in ipairs(plan.changes) do
    local prefix = "change." .. index .. "."
    outbox[#outbox + 1] = prefix .. "action"
    outbox[#outbox + 1] = change.action
    outbox[#outbox + 1] = prefix .. "member"
    outbox[#outbox + 1] = items[index].memberJson
    if change.previous then
      -- Для Leave без data сохраняем последнее состояние участника целиком.
      outbox[#outbox + 1] = prefix .. "previous"
      outbox[#outbox + 1] = type(change.previous) == "number"
        and items[change.previous].memberJson or change.previous
    end
  end

  local members = {}
  for index, entry in ipairs(plan.members) do
    members[index] = {
      member_field = entry.member_field, connection_member = entry.connection_member,
      member_json = entry.after and items[entry.after].memberJson or false,
    }
  end
  return {
    members = members, presence_revision = counters.presence_revision, updated_at = timestamp,
    outbox = outbox, outcome = {"committed", outbox},
  }
end
