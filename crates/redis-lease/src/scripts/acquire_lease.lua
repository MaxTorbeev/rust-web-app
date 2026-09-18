-- Захватывает lease или продлевает его для текущего владельца.
-- Возвращает {1, fence}, {2, remaining_ms} или Redis error reply.
-- Fence передаётся десятичной строкой, без преобразования в Lua-число.
local function acquire_lease(key, fence_key, owner_value, ttl_ms)
  if not owner_value or string.sub(owner_value, 1, 6) ~= "lease:" or string.len(owner_value) <= 6 then
    return redis.error_reply("ERR invalid lease owner value")
  end

  local current = redis.call("GET", key)

  if not current then
    -- Ошибка INCR не должна оставлять созданный lease. Если последующий SET
    -- завершится ошибкой, пропуск номера fence допустим: счётчик не откатываем.
    redis.call("INCR", fence_key)
    -- Ответ INCR в Lua может потерять точность. GET возвращает точную строку.
    local fence = redis.call("GET", fence_key)
    -- SET с PX создаёт lease сразу с TTL: отдельные SET и PEXPIRE оставили бы
    -- окно, в котором lease мог сохраниться навсегда после сбоя владельца.
    redis.call("SET", key, owner_value .. ":" .. fence, "PX", ttl_ms)
    return {1, fence}
  end

  -- Fence — последний сегмент и только цифры; владелец может содержать `:`,
  -- поэтому разбираем с конца. Значение другого вида этому крейту не принадлежит:
  -- это повреждение или чужой ключ, а не «занято».
  local current_owner, current_fence = string.match(current, "^(lease:.+):([1-9]%d*)$")

  if not current_owner then
    return redis.error_reply("ERR invalid lease value")
  end

  local remaining_ms = redis.call("PTTL", key)

  -- PTTL возвращает -1 для ключа без TTL и -2 для отсутствующего ключа. Lease без
  -- TTL никогда не истечёт — это повреждённое состояние, а не «занято».
  if remaining_ms < 0 then
    return redis.error_reply("ERR lease state has no valid TTL")
  end

  -- Проверяем целостность до PEXPIRE: ошибка не должна продлевать lease.
  local stored_fence = redis.call("GET", fence_key)

  if not stored_fence then
    return redis.error_reply("ERR lease has no fence")
  end

  if stored_fence ~= current_fence then
    return redis.error_reply("ERR lease fence mismatch")
  end

  if current_owner == owner_value then
    redis.call("PEXPIRE", key, ttl_ms)
    return {1, current_fence}
  end

  return {2, remaining_ms}
end
