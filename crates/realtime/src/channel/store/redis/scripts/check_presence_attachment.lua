-- Вызывать после проверки connection state (status/generation) и lookup ledger.
-- Перед этим фрагментом должен быть определён read_attachment.
-- actor содержит connectionId/nodeInstance из проверенной Rust-команды.
-- Успех: attachment, nil. Доменный отказ: attachment/false, {"rejected", code}.
-- Ошибка хранилища: nil, {0, code, message}.
local function check_presence_attachment(attachments_key, connection_field, actor, connection_exists)
  local attachment, failure = read_attachment(
    attachments_key, connection_field, actor, connection_exists
  )
  if failure then return nil, failure end
  if not attachment then return false, {"rejected", "notAttached"} end
  for _, mode in ipairs(attachment.effectiveModes) do
    if mode == "presence" then return attachment, nil end
  end
  return attachment, {"rejected", "presenceModeNotEnabled"}
end
