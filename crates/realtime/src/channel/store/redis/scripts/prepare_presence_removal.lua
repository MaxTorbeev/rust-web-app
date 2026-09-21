-- Base64url UTF-8 без padding, как protocol::segment в Rust.
-- Нужен для проверки соответствия clientId полю K.U и ссылке C.U.
-- Основной attach использует его также для проверки поколения команды.
local function member_segment(value)
  local alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
  local encoded = {}
  for offset = 1, #value, 3 do
    local a, b, c = string.byte(value, offset, offset + 2)
    local bits = a * 65536 + (b or 0) * 256 + (c or 0)
    local length = c and 4 or (b and 3 or 2)
    for position = 1, length do
      local index = math.floor(bits / 2 ^ (24 - 6 * position)) % 64 + 1
      encoded[#encoded + 1] = string.sub(alphabet, index, index)
    end
  end
  return table.concat(encoded)
end

-- Подготавливает удаление всех members одного attachment, без записей.
-- Вызывать после проверки lease и владельца attachment.
-- Attach вызывает helper при потере Presence; detach — при удалении attachment.
-- attachment — проверенная identity команды для сравнения с участниками.
-- members — массив field/{payload, revision, timestamp} из read_presence_members,
-- выполненного вызывающим transition в этом же Lua-вызове после проверки lease.
-- Массив не изменяется: он же используется для snapshot, исключая поля из
-- плана удаления. Повторное чтение HASH для snapshot не требуется.
-- connection_members_key — SET соединения;
-- channel_segment = C, connection_segment = K из схемы Redis v1.
-- indexed — необязательный набор ссылок C.U только этого канала, прочитанный
-- в том же Lua-вызове. Проверенные ссылки удаляются из переданной таблицы.
--
-- Успех: список {member_field, connection_member, member_json}, nil.
-- Порядок не определён; #списка — число удаляемых members.
-- Rust сортирует участников по client_id перед формированием Leave.
-- Отказ: nil, {0, code, message}; вызывающий скрипт возвращает его до записей.
-- member_json — payload из Rust, который сохраняется побайтно:
-- иначе большие целые в data могут потерять точность.
local function prepare_presence_removal(
  members, connection_members_key, channel_segment, connection_segment, attachment, indexed
)
  local channel_prefix = channel_segment .. "."
  local connection_prefix = connection_segment .. "."
  if not indexed then
    indexed = {}
    for _, reference in ipairs(redis.call("SMEMBERS", connection_members_key)) do
      if string.sub(reference, 1, #channel_prefix) == channel_prefix then
        indexed[reference] = true
      end
    end
  end

  local removed = {}
  -- Проверяем уже прочитанные members, включая записи без обратной ссылки.
  for index = 1, #members, 2 do
    local field, raw = members[index], members[index + 1][1]
    if string.sub(field, 1, #connection_prefix) == connection_prefix then
      local ok, member = pcall(cjson.decode, raw)
      if not ok or type(member) ~= "table"
        or type(member.clientId) ~= "string"
        or type(member.nodeInstance) ~= "table"
        or type(member.nodeInstance.nodeId) ~= "string"
        or type(member.nodeInstance.bootGeneration) ~= "string" then
        return nil, {0, "corrupt_state", "invalid stored Presence member identity"}
      end

      local client_segment = member_segment(member.clientId)
      if member.connectionId ~= attachment.connectionId
        or field ~= connection_prefix .. client_segment then
        return nil, {0, "corrupt_state", "Presence member identity does not match its hash field"}
      end
      if member.nodeInstance.nodeId ~= attachment.nodeInstance.nodeId
        or member.nodeInstance.bootGeneration ~= attachment.nodeInstance.bootGeneration then
        return nil, {0, "generation_mismatch", "Presence member belongs to another node generation"}
      end

      local reference = channel_prefix .. client_segment
      if not indexed[reference] then
        return nil, {0, "corrupt_state", "Presence member is missing its connection index entry"}
      end
      indexed[reference] = nil
      removed[#removed + 1] = {
        member_field = field,
        connection_member = reference,
        member_json = raw,
      }
    end
  end

  if next(indexed) ~= nil then
    return nil, {0, "corrupt_state", "connection index references a missing Presence member"}
  end
  return removed, nil
end
