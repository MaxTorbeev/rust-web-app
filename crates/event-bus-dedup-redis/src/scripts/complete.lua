-- Атомарно заменяет принадлежащий worker-у lease на completed marker.
--
-- KEYS[1]: полный Redis key события.
--
-- ARGV[1]: ожидаемое значение lease в формате `lease:<token>`.
--          Оно должно полностью совпасть со значением, сохранённым claim.
-- ARGV[2]: TTL completed marker в целых положительных миллисекундах.
--
-- Ответы:
--   1 — lease всё ещё принадлежал вызывающему worker и был завершён;
--   0 — key отсутствует, уже completed либо содержит lease другого worker.
--       Rust-код преобразует этот ответ в DedupStoreError::LeaseLost.
--
-- Сравнение token и SET выполняются одним атомарным скриптом. Просроченный
-- worker поэтому не может завершить более новый lease другого worker-а.

local key = KEYS[1]
local expected_lease = ARGV[1]
local completed_ttl_ms = ARGV[2]

-- Защищаем completed marker от ошибочного вызова complete с произвольным
-- expected value. Сравнивать разрешено только полноценное lease-значение.
local has_valid_lease_format =
  expected_lease
  and string.sub(expected_lease, 1, 6) == "lease:"
  and string.len(expected_lease) > 6

if not has_valid_lease_format then
  return redis.error_reply("ERR invalid expected dedup lease value")
end

local current = redis.call("GET", key)

-- Сравниваем полное значение `lease:<token>`. Отсутствующий key, completed
-- marker и чужой token одинаково означают, что вызывающий больше не владеет
-- lease. Никакое состояние в этой ветке не изменяется.
if current ~= expected_lease then
  return 0
end

-- Совпавший lease также обязан быть временным. Persistent lease является
-- повреждённым состоянием backend-а, а не корректным владением.
local remaining_ttl_ms = redis.call("PTTL", key)

if remaining_ttl_ms < 0 then
  return redis.error_reply("ERR dedup lease has no valid TTL")
end

-- Один SET одновременно заменяет lease на completed marker и устанавливает
-- новый TTL. После этого redelivery увидит Completed, а не запустит handler
-- повторно. Невалидный ARGV[2] приводит к ошибке самого SET без изменения key.
redis.call("SET", key, "completed", "PX", completed_ttl_ms)

return 1
