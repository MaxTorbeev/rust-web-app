-- Сохранённые serial имеют тот же формат, что protocol::operation_serial в Rust.
local function valid_operation_serial(value)
  return type(value) == "string" and #value == 20
    and string.match(value, "^%d+$") ~= nil and value <= "18446744073709551615"
end

