-- Counters строками и флаги категорий: сериализация JSON не нужна.
local function append_occupancy_fields(fields, counters)
  for _, name in ipairs({
    "connections", "publishers", "subscribers", "presence_connections",
    "presence_subscribers", "presence_members",
  }) do
    fields[#fields + 1] = name
    fields[#fields + 1] = counters.metrics[name]
  end
  for _, name in ipairs(counters.changed_categories) do
    fields[#fields + 1] = "changed." .. name
    fields[#fields + 1] = "1"
  end
  for _, name in ipairs(counters.zero_boundary_categories) do
    fields[#fields + 1] = "boundary." .. name
    fields[#fields + 1] = "1"
  end
end
