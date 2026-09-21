-- Подготавливает данные snapshot, transition и outbox без Redis I/O.
-- members — массив field/{payload, revision, timestamp} из read_presence_members.
-- removed/counters — планы предыдущих шагов.
-- removed построен из этого же массива, который с тех пор не изменялся.
-- channel_json/attachment_json/event_id приходят из проверенной Rust-команды;
-- now_ms — время Redis, полученное при проверке lease.
-- JSON участников и attachment передаётся без разбора и перекодирования.
-- Rust собирает публичные типы и сортирует участников из этих данных.
-- Успех: {snapshot, transition, outbox}, nil; outbox = false для no-op.
-- snapshot/transition — массивы для RESP, outbox — пары field/value для XADD.
-- Отказ: nil, {0, code, message}. Вызывать до любых записей.

local function prepare_attach_result(members, removed, counters, channel_json, attachment_json, event_id, now_ms)
  local removal_fields = {}
  for _, entry in ipairs(removed) do
    removal_fields[entry.member_field] = true
  end
  local retained = {}
  for index = 1, #members, 2 do
    local field, raw = members[index], members[index + 1]
    if not removal_fields[field] then
      retained[#retained + 1] = raw
    end
  end
  if string.format("%.0f", #retained) ~= counters.metrics.presence_members then
    return nil, {0, "corrupt_state", "Presence member count does not match the prepared snapshot"}
  end

  local metrics = {}
  for _, field in ipairs({
    "connections", "publishers", "subscribers", "presence_connections",
    "presence_subscribers", "presence_members",
  }) do
    metrics[#metrics + 1] = field
    metrics[#metrics + 1] = counters.metrics[field]
  end
  local snapshot = {retained, counters.presence_revision, counters.occupancy_version, metrics}
  if #counters.changed_categories == 0 then
    return {
      snapshot = snapshot,
      transition = {"unchanged", counters.occupancy_version},
      outbox = false,
    }, nil
  end

  local outbox = prepare_removal_outbox(
    {"format", "attach.v2", "attachment_json", attachment_json},
    removed, counters, channel_json, event_id, now_ms
  )
  return {
    snapshot = snapshot,
    transition = {"changed", outbox},
    outbox = outbox,
  }, nil
end
