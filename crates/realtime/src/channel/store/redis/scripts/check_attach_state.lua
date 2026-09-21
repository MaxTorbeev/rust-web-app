-- Проверяет сохранённое состояние перед индивидуальным attach, без записей.
-- Вызывать после check_node_lease внутри того же mutation-скрипта.
-- Перед этим фрагментом должен быть определён read_attachment.
-- attachment — уже проверенные параметры нового attachment из команды Rust;
-- connection_field — закодированный connection ID (K), generation — его G.
-- connection_exists — результат TYPE connection_key ~= "none" в этом же вызове.
--
-- Успех: предыдущий attachment (или false, если его нет), nil.
-- Отказ: nil, {0, code, message}; вызывающий скрипт возвращает отказ до записей.
-- Декодированный attachment нужен для сравнения владельца и расчёта counters,
-- а не для повторной сериализации: cjson может округлять числовые метаданные.
local function check_attach_state(connection_key, attachments_key, connection_field, generation, attachment, connection_exists)
  local state = redis.call("HMGET", connection_key, "status", "generation")

  if connection_exists then
    -- В том числе отклоняем HASH без обязательных полей.
    if (state[1] ~= "open" and state[1] ~= "closed") or not state[2] or state[2] == "" then
      return nil, {0, "corrupt_state", "invalid connection status or generation"}
    end
    if state[1] == "closed" then
      return nil, {0, "connection_closed", "closed connection cannot attach"}
    end
    if state[2] ~= generation then
      return nil, {0, "generation_mismatch", "connection belongs to another node generation"}
    end
  end

  return read_attachment(attachments_key, connection_field, attachment, connection_exists)
end
