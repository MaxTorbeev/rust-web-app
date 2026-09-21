-- Поиск в ledger внутри apply_presence, до проверки attachment.
-- Вызывать после проверки lease и поколения connection state.
-- serial приходит из Rust как 20 цифр; highest_serial уже проверен transition.
-- closed — признак закрытого connection state. Сам helper ничего не записывает.
-- Результат: {"replayed", record_fields}, {"rejected", code} или nil для новой операции.
-- Второе значение — ошибка хранилища {0, code, message}, если запись повреждена.

local function lookup_presence_operation(operation_key, order_key, serial, fingerprint, highest_serial, closed)
  local saved = redis.call("HGETALL", operation_key)
  local indexed = redis.call("ZSCORE", order_key, serial)
  if (#saved > 0) ~= (indexed ~= false) or (indexed and indexed ~= "0") then
    return nil, {0, "corrupt_state", "Presence operation does not match its order index"}
  end
  if #saved > 0 then
    local saved_fingerprint, result
    for index = 1, #saved, 2 do
      if saved[index] == "fingerprint" then saved_fingerprint = saved[index + 1] end
      if saved[index] == "result" then result = saved[index + 1] end
    end
    if not saved_fingerprint or (result ~= "committed" and result ~= "rejected") then
      return nil, {0, "corrupt_state", "invalid Presence operation record"}
    end
    if saved_fingerprint ~= fingerprint then
      return {"rejected", "conflictingReplay"}, nil
    end
    return {"replayed", saved}, nil
  end

  if highest_serial then
    if serial <= highest_serial then
      local lowest = redis.call("ZRANGE", order_key, 0, 0)[1]
      if lowest and not valid_operation_serial(lowest) then
        return nil, {0, "corrupt_state", "invalid Presence operation order serial"}
      end
      if not lowest or serial < lowest then
        return {"rejected", "staleOperation"}, nil
      end
    end
  end

  if closed then return {"rejected", "connectionClosed"}, nil end
  return nil, nil
end
