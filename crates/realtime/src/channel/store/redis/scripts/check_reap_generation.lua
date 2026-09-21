-- Общая проверка перед чтением, очисткой и финализацией поколения. Без записей.
local function check_reap_generation(reaper_key, deadlines_key, cleanup_key, target_key,
  reaper_token, reaper_generation, cleanup_token, target_generation)
  local now_ms, failure = check_node_lease(reaper_key, deadlines_key, reaper_token, reaper_generation)
  if failure then return nil, failure end

  local cleanup_owner = string.match(cleanup_token, "^(lease:.+):[1-9]%d*$")
  if cleanup_owner ~= "lease:" .. reaper_generation then
    return nil, {0, "invalid_request", "cleanup token does not belong to the reaper generation"}
  end
  if not holds_lease(cleanup_key, cleanup_token) then
    return nil, {0, "lease_lost", "cleanup lease is expired or belongs to another token"}
  end

  local deadline_value = redis.call("ZSCORE", deadlines_key, target_generation)
  if not deadline_value then
    return nil, {0, "generation_mismatch", "target generation deadline is missing"}
  end
  local deadline = tonumber(deadline_value)
  if not deadline or deadline < 0 or deadline > 9007199254740991 or deadline ~= math.floor(deadline) then
    return nil, {0, "corrupt_state", "target generation deadline must be exact Unix milliseconds"}
  end
  if deadline > now_ms then
    return nil, {0, "generation_active", "target generation deadline has not expired"}
  end
  -- Новый boot того же node ID не мешает очистке старого поколения.
  local target_lease = redis.call("GET", target_key)
  if target_lease and string.match(target_lease, "^(lease:.+):[1-9]%d*$") == "lease:" .. target_generation then
    return nil, {0, "generation_active", "target generation still holds its node lease"}
  end

  return now_ms, nil
end
