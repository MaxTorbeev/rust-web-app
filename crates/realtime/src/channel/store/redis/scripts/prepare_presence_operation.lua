-- Пары field/value для HASH одной операции. Вызывать до любых записей.
-- outcome: {"committed", outbox} либо {"rejected", code[, client_id]}.
local function prepare_presence_operation(fingerprint, outcome)
  local fields = {"fingerprint", fingerprint, "result", outcome[1]}
  if outcome[1] == "committed" then
    for _, value in ipairs(outcome[2]) do fields[#fields + 1] = value end
  else
    fields[#fields + 1] = "code"
    fields[#fields + 1] = outcome[2]
    if outcome[3] then
      fields[#fields + 1] = "client_id"
      fields[#fields + 1] = outcome[3]
    end
  end
  return fields
end
