-- Возвращает 0 или 1 для каждого из пяти счётчиков Occupancy:
-- 1 означает, что этот individual attachment увеличивает счётчик канала на 1.
-- При отсутствии attachment (false/nil) все пять counters равны нулю.
-- Формат и accounting проверяются до вызова; функция не обращается к Redis.
-- presence_members считается отдельно по записям участников Presence.
-- Transition вычисляет эти значения для старого и нового attachment один раз
-- и передаёт таблицы в prepare_attach_counters, не изменяющий их.
local function attachment_occupancy(attachment)
  local counters = {
    connections = 0,
    publishers = 0,
    subscribers = 0,
    presence_connections = 0,
    presence_subscribers = 0,
  }

  if not attachment then
    return counters
  end

  counters.connections = 1
  for _, mode in ipairs(attachment.effectiveModes) do
    -- Даже при повторении режима соединение учитывается в счётчике один раз.
    if mode == "publish" then
      counters.publishers = 1
    elseif mode == "subscribe" then
      counters.subscribers = 1
    elseif mode == "presence" then
      counters.presence_connections = 1
    elseif mode == "presenceSubscribe" then
      counters.presence_subscribers = 1
    end
  end

  return counters
end
