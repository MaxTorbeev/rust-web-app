-- Продлевает lease, только если сохранённый token совпадает с переданным.
-- Возвращает 1 при продлении, 0 при потере владения.
local function renew_lease(key, token_value, ttl_ms)
  if redis.call("GET", key) ~= token_value then
    return 0
  end

  redis.call("PEXPIRE", key, ttl_ms)
  return 1
end
