-- Фрагмент для mutation-скриптов; перед ним должен быть определён holds_lease.
-- Проверка вызывается внутри того же Lua-вызова, что и изменение состояния.
--
-- Успех: now_ms, nil — время Redis для последующего canonical event.
-- Отказ: nil, {0, code, message} — вызывающий скрипт должен вернуть этот ответ
-- до любых записей. Сам helper ничего не изменяет и не продлевает lease.
-- Неверные Redis types отклоняются командами GET/ZSCORE с ошибкой WRONGTYPE.
--
-- Использование в transition:
-- local now_ms, rejection = check_node_lease(KEYS[1], KEYS[2], ARGV[1], ARGV[2])
-- if rejection then return rejection end
-- Затем проверки состояния и commit в этом же скрипте.
local function check_node_lease(lease_key, deadlines_key, token_value, generation)
  if type(token_value) ~= "string" or type(generation) ~= "string" then
    return nil, {0, "invalid_request", "node lease token and generation are required"}
  end

  -- Fence остаётся строкой: Lua number не представляет все значения точно.
  local token_owner = string.match(token_value, "^(lease:.+):[1-9]%d*$")
  if token_owner ~= "lease:" .. generation then
    return nil, {0, "invalid_request", "node lease token does not match generation"}
  end

  if not holds_lease(lease_key, token_value) then
    return nil, {0, "lease_lost", "node lease is expired or belongs to another token"}
  end

  local deadline_value = redis.call("ZSCORE", deadlines_key, generation)
  if not deadline_value then
    return nil, {0, "lease_lost", "node generation deadline is missing"}
  end

  local deadline_ms = tonumber(deadline_value)
  if not deadline_ms or deadline_ms < 0 or deadline_ms > 9007199254740991
    or deadline_ms ~= math.floor(deadline_ms) then
    return nil, {0, "corrupt_state", "node generation deadline must be exact Unix milliseconds"}
  end

  local time = redis.call("TIME")
  local now_ms = tonumber(time[1]) * 1000 + math.floor(tonumber(time[2]) / 1000)
  if deadline_ms <= now_ms then
    return nil, {0, "lease_lost", "node generation deadline has expired"}
  end

  return now_ms, nil
end
