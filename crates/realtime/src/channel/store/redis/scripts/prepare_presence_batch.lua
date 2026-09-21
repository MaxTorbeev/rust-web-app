-- Читает только затронутого участника и проверяет его обратную ссылку.
-- Исходный JSON возвращается без перекодирования пользовательских данных.
local function read_presence_member(members_key, connection_members_key, field, reference, client_id, attachment)
  local stored = redis.call("HMGET", members_key, field, field .. ":revision", field .. ":updated_at")
  local raw = stored[1]
  if (raw ~= false) ~= (stored[2] ~= false) or (raw ~= false) ~= (stored[3] ~= false) then
    return nil, {0, "corrupt_state", "incomplete Presence member record"}
  end
  local indexed = redis.call("SISMEMBER", connection_members_key, reference) == 1
  if (raw ~= false) ~= indexed then
    return nil, {0, "corrupt_state", "Presence member does not match its connection index"}
  end
  if not raw then return false, nil end

  local ok, member = pcall(cjson.decode, raw)
  if not ok or type(member) ~= "table" or type(member.nodeInstance) ~= "table"
    or member.connectionId ~= attachment.connectionId or member.clientId ~= client_id then
    return nil, {0, "corrupt_state", "invalid stored Presence member identity"}
  end
  if member.nodeInstance.nodeId ~= attachment.nodeInstance.nodeId
    or member.nodeInstance.bootGeneration ~= attachment.nodeInstance.bootGeneration then
    return nil, {0, "generation_mismatch", "Presence member belongs to another node generation"}
  end
  return raw, nil
end

-- Планирует непустой batch после проверки attachment, без записей.
-- items — JSON-массив из Rust encode_presence_items; поля входа уже типизированы.
-- Успех: {members, changes, member_delta}, nil. Отказ: nil, failure.
-- after/previous: false, исходный member JSON или индекс элемента items (с 1).
local function prepare_presence_batch(members_key, connection_members_key, channel_segment, connection_segment, attachment, items)
  local by_client, members, changes, member_delta = {}, {}, {}, 0
  for index, item in ipairs(items) do
    if item.clientId == cjson.null then
      return nil, {"rejected", "unidentifiedConnection"}
    end
    if not item.allowed then
      return nil, {"rejected", "clientIdNotAllowed", item.clientId}
    end

    local entry = by_client[item.clientId]
    if not entry then
      local field = connection_segment .. "." .. item.clientSegment
      local reference = channel_segment .. "." .. item.clientSegment
      local raw, failure = read_presence_member(
        members_key, connection_members_key, field, reference, item.clientId, attachment
      )
      if failure then return nil, failure end
      entry = {member_field = field, connection_member = reference, after = raw}
      by_client[item.clientId] = entry
      members[#members + 1] = entry
    end

    local previous, action = entry.after, item.action
    if action ~= "enter" and previous == false then
      return nil, {"rejected", "invalidMemberState"}
    end
    if action == "enter" and previous ~= false then action = "update" end

    if action == "leave" then
      entry.after = false
      member_delta = member_delta - 1
    else
      entry.after = index
      if previous == false then member_delta = member_delta + 1 end
    end
    changes[#changes + 1] = {
      action = action,
      previous = action == "leave" and not item.hasData and previous or false,
    }
  end

  return {members = members, changes = changes, member_delta = member_delta}, nil
end
