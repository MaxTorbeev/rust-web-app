-- Читает и проверяет индивидуальный attachment без повторного чтения connection state.
-- actor содержит connectionId/nodeInstance; connection_exists получен вызывающим скриптом.
-- Успех: attachment (или false), nil. Ошибка: nil, {0, code, message}.
local function read_attachment(attachments_key, connection_field, actor, connection_exists)
  local saved = redis.call("HGET", attachments_key, connection_field)
  if not saved then return false, nil end
  if not connection_exists then
    return nil, {0, "corrupt_state", "attachment exists without connection state"}
  end

  local ok, attachment = pcall(cjson.decode, saved)
  if not ok or type(attachment) ~= "table"
    or type(attachment.connectionId) ~= "string"
    or type(attachment.nodeInstance) ~= "table"
    or type(attachment.nodeInstance.nodeId) ~= "string"
    or type(attachment.nodeInstance.bootGeneration) ~= "string" then
    return nil, {0, "corrupt_state", "invalid stored attachment identity"}
  end
  if attachment.connectionId ~= actor.connectionId then
    return nil, {0, "corrupt_state", "attachment connection does not match its hash field"}
  end
  if attachment.nodeInstance.nodeId ~= actor.nodeInstance.nodeId
    or attachment.nodeInstance.bootGeneration ~= actor.nodeInstance.bootGeneration then
    return nil, {0, "generation_mismatch", "attachment belongs to another node generation"}
  end
  if attachment.accounting ~= "individual" then
    return nil, {0, "corrupt_state", "stored attachment must use individual accounting"}
  end

  local modes = attachment.effectiveModes
  if type(modes) ~= "table" or #modes == 0 then
    return nil, {0, "corrupt_state", "stored attachment requires effective modes"}
  end
  for index, mode in pairs(modes) do
    if type(index) ~= "number"
      or (mode ~= "subscribe" and mode ~= "publish" and mode ~= "presence" and mode ~= "presenceSubscribe") then
      return nil, {0, "corrupt_state", "invalid stored attachment effective modes"}
    end
  end

  return attachment, nil
end
