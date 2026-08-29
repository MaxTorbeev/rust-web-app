-- Атомарно освобождает lease после ошибки или timeout handler-а.
--
-- KEYS[1]: полный Redis key события.
--
-- ARGV[1]: ожидаемое значение lease в формате `lease:<token>`.
--
-- Ответы:
--   1 — вызывающий worker владел lease, key удалён;
--   0 — key отсутствует, уже completed либо принадлежит другому worker.
--       Rust-код преобразует этот ответ в DedupStoreError::LeaseLost.
--
-- Проверка token и DEL находятся в одном атомарном скрипте. Старый worker не
-- может удалить completed marker или новый lease, созданный после истечения
-- его собственного TTL.

local key = KEYS[1]
local expected_lease = ARGV[1]

-- Без этой проверки ошибочно переданный expected value `completed` совпал бы
-- с completed marker и позволил бы удалить его. Release принимает только
-- полноценное значение `lease:<token>`.
local has_valid_lease_format =
  expected_lease
  and string.sub(expected_lease, 1, 6) == "lease:"
  and string.len(expected_lease) > 6

if not has_valid_lease_format then
  return redis.error_reply("ERR invalid expected dedup lease value")
end

local current = redis.call("GET", key)

-- Удаляем только точное значение `lease:<token>`, полученное этим worker-ом.
-- Любое несовпадение оставляем без изменений.
if current ~= expected_lease then
  return 0
end

redis.call("DEL", key)

return 1
