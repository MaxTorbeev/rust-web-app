-- KEYS: node lease, deadlines, publisher lease, outbox.
-- ARGV: node token, generation, publisher token, read/ack, count/entry ID.
if #KEYS ~= 4 or #ARGV ~= 5 then return redis.error_reply("invalid outbox arguments") end
local _, failure = check_node_lease(KEYS[1], KEYS[2], ARGV[1], ARGV[2])
if failure then return failure end
if not holds_lease(KEYS[3], ARGV[3]) then return {0, "lease_lost", "publisher lease is lost"} end
if ARGV[4] == "read" then
  return {1, redis.call("XRANGE", KEYS[4], "-", "+", "COUNT", ARGV[5])}
elseif ARGV[4] == "ack" then
  redis.call("XDEL", KEYS[4], ARGV[5])
  return {1, {}}
end
return redis.error_reply("unknown outbox operation")
