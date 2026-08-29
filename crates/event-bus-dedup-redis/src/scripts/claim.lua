-- Атомарно пытается получить lease на обработку одного deduplication key.
--
-- KEYS[1]: полный Redis key события:
--          <app>.<environment>.event-bus.v1.dedup.<scope>.<event-id>
--
-- ARGV[1]: новое значение lease в формате `lease:<token>`.
--          Token генерируется вызывающим Rust-кодом отдельно для каждой
--          попытки claim.
-- ARGV[2]: lease TTL в целых положительных миллисекундах. Rust-код обязан
--          округлить Duration вверх и проверить диапазон до вызова скрипта.
--
-- Ответы:
--   {1}      — ключ отсутствовал, новый lease создан;
--   {2}      — событие уже успешно обработано;
--   {3, ttl} — другой worker владеет lease, ttl содержит оставшееся время
--              этого lease в миллисекундах.
--
-- Скрипт выполняется Redis атомарно. Между GET, SET и PTTL другой worker не
-- может изменить этот key, поэтому два конкурентных claim не получат lease
-- одновременно.

local key = KEYS[1]
local new_lease = ARGV[1]
local lease_ttl_ms = ARGV[2]

-- Не записываем состояние, которое последующие complete/release не смогут
-- безопасно распознать как lease. Проверка не валидирует конкретный формат
-- token и позволяет заменить UUID другим непустым token в будущей версии.
local has_valid_lease_format =
  new_lease
  and string.sub(new_lease, 1, 6) == "lease:"
  and string.len(new_lease) > 6

if not has_valid_lease_format then
  return redis.error_reply("ERR invalid dedup lease value")
end

local current = redis.call("GET", key)

-- Отсутствующий key означает, что событие сейчас свободно для обработки.
-- SET с PX создаёт lease сразу с TTL: отдельные SET и PEXPIRE оставили бы
-- окно, в котором lease мог сохраниться навсегда после сбоя worker-а.
if not current then
  redis.call("SET", key, new_lease, "PX", lease_ttl_ms)
  return {1}
end

-- Любое сохранённое состояние обязано иметь TTL. Значение без TTL означает
-- повреждённый protocol state; продолжать обработку как обычно небезопасно.
local remaining_ttl_ms = redis.call("PTTL", key)

-- PTTL возвращает -1 для key без TTL и -2 для отсутствующего key. Второй
-- вариант в нормальном атомарном выполнении недостижим, но также считается
-- ошибкой состояния.
if remaining_ttl_ms < 0 then
  return redis.error_reply("ERR dedup state has no valid TTL")
end

-- Completed marker запрещает повторный запуск handler-а до истечения
-- completed TTL. Claim не обновляет этот TTL.
if current == "completed" then
  return {2}
end

-- Действующий lease принадлежит другому worker. Возвращаем именно оставшийся
-- TTL и не продлеваем lease повторным claim.
if string.sub(current, 1, 6) == "lease:" and string.len(current) > 6 then
  return {3, remaining_ttl_ms}
end

-- Неизвестное значение нельзя трактовать ни как свободный key, ни как
-- completed: оба варианта могли бы привести к повторной обработке события.
return redis.error_reply("ERR unknown dedup state")
