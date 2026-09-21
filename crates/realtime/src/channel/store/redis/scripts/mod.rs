//! Lua-скрипты Redis-адаптера состояния каналов.

use const_format::concatcp;
use redis_lease::{LUA_ACQUIRE_LEASE, LUA_HOLDS_LEASE, LUA_RENEW_LEASE};

/// Выбирает истёкшие поколения и их metadata по Redis TIME без записей.
pub(super) const EXPIRED_GENERATIONS: &str = include_str!("expired_generations.lua");

pub(super) const OUTBOX: &str = concatcp!(LUA_CHECK_NODE_LEASE, "\n", include_str!("outbox.lua"));

/// Читает участников, версии и Occupancy counters одним вызовом без записей.
pub(super) const SNAPSHOT: &str = concatcp!(
  LUA_READ_CHANNEL_STATE,
  "\n",
  LUA_READ_PRESENCE_MEMBERS,
  "\n",
  include_str!("snapshot.lua"),
);

const LUA_READ_CHANNEL_STATE: &str = include_str!("read_channel_state.lua");

/// Проверка владения для встраивания в mutation-скрипты, а не отдельного вызова.
pub(super) const LUA_CHECK_NODE_LEASE: &str =
  concatcp!(LUA_HOLDS_LEASE, "\n", include_str!("check_node_lease.lua"),);

/// Проверка connection state и attachment для встраивания после проверки lease.
pub(super) const LUA_CHECK_ATTACH_STATE: &str = concatcp!(
  LUA_READ_ATTACHMENT,
  "\n",
  include_str!("check_attach_state.lua"),
);

const LUA_READ_ATTACHMENT: &str = include_str!("read_attachment.lua");

const LUA_OPERATION_SERIAL: &str = include_str!("operation_serial.lua");

/// Поиск повтора или отказа ledger для встраивания в apply_presence.
pub(super) const LUA_LOOKUP_PRESENCE_OPERATION: &str = concatcp!(
  LUA_OPERATION_SERIAL,
  "\n",
  include_str!("lookup_presence_operation.lua"),
);

/// Проверяет attachment и готовит Presence batch без записей.
pub(super) const LUA_PREPARE_PRESENCE_BATCH: &str = concatcp!(
  LUA_READ_ATTACHMENT,
  "\n",
  include_str!("check_presence_attachment.lua"),
  "\n",
  include_str!("prepare_presence_batch.lua"),
);

/// Рассчитывает счётчики и версии успешно подготовленного Presence batch.
pub(super) const LUA_PREPARE_PRESENCE_COUNTERS: &str = concatcp!(
  LUA_READ_CHANNEL_STATE,
  "\n",
  include_str!("prepare_presence_counters.lua"),
);

const LUA_READ_PRESENCE_MEMBERS: &str = include_str!("read_presence_members.lua");
const LUA_APPEND_OCCUPANCY_FIELDS: &str = include_str!("append_occupancy_fields.lua");

/// Готовит поля HASH одной операции для атомарной записи вместе с состоянием.
pub(super) const LUA_PREPARE_PRESENCE_OPERATION: &str =
  include_str!("prepare_presence_operation.lua");

/// Готовит записи участников и неизменяемые данные результата и outbox.
pub(super) const LUA_PREPARE_PRESENCE_RESULT: &str = concatcp!(
  LUA_APPEND_OCCUPANCY_FIELDS,
  "\n",
  include_str!("prepare_presence_result.lua"),
);

/// Захват node lease вместе с регистрацией поколения и его deadline.
pub(super) const CLAIM_NODE: &str =
  concatcp!(LUA_ACQUIRE_LEASE, "\n", include_str!("claim_node.lua"),);

/// Продление node lease вместе с deadline его поколения.
pub(super) const RENEW_NODE: &str =
  concatcp!(LUA_RENEW_LEASE, "\n", include_str!("renew_node.lua"),);

/// Сохраняет attachment и возвращает snapshot, атомарно записывая
/// изменения состояния и данные события в outbox.
pub(super) const ATTACH_AND_SNAPSHOT: &str = concatcp!(
  LUA_CHECK_NODE_LEASE,
  "\n",
  LUA_CHECK_ATTACH_STATE,
  "\n",
  include_str!("attachment_occupancy.lua"),
  "\n",
  LUA_READ_PRESENCE_MEMBERS,
  "\n",
  include_str!("prepare_presence_removal.lua"),
  "\n",
  LUA_READ_CHANNEL_STATE,
  "\n",
  include_str!("prepare_attach_counters.lua"),
  "\n",
  LUA_APPEND_OCCUPANCY_FIELDS,
  "\n",
  include_str!("prepare_removal_outbox.lua"),
  "\n",
  include_str!("prepare_attach_result.lua"),
  "\n",
  include_str!("attach_and_snapshot.lua"),
);

/// Проверяет Presence batch и атомарно фиксирует state, ledger и outbox.
pub(super) const APPLY_PRESENCE: &str = concatcp!(
  LUA_CHECK_NODE_LEASE,
  "\n",
  LUA_LOOKUP_PRESENCE_OPERATION,
  "\n",
  LUA_PREPARE_PRESENCE_BATCH,
  "\n",
  LUA_PREPARE_PRESENCE_COUNTERS,
  "\n",
  LUA_PREPARE_PRESENCE_RESULT,
  "\n",
  LUA_PREPARE_PRESENCE_OPERATION,
  "\n",
  include_str!("apply_presence.lua"),
);

// Общая подготовка и commit удаления одного канала для detach/disconnect/reaper.
const LUA_DETACH_HELPERS: &str = concatcp!(
  LUA_CHECK_NODE_LEASE,
  "\n",
  LUA_READ_ATTACHMENT,
  "\n",
  include_str!("attachment_occupancy.lua"),
  "\n",
  LUA_READ_PRESENCE_MEMBERS,
  "\n",
  include_str!("prepare_presence_removal.lua"),
  "\n",
  LUA_READ_CHANNEL_STATE,
  "\n",
  include_str!("prepare_attach_counters.lua"),
  "\n",
  LUA_APPEND_OCCUPANCY_FIELDS,
  "\n",
  include_str!("prepare_removal_outbox.lua"),
  "\n",
  include_str!("prepare_detach.lua"),
);

/// Удаляет attachment и его members, фиксируя counters и событие в outbox.
pub(super) const DETACH: &str = concatcp!(LUA_DETACH_HELPERS, "\n", include_str!("detach.lua"));

const LUA_CLOSE_CONNECTION: &str = concatcp!(
  LUA_DETACH_HELPERS,
  "\n",
  LUA_OPERATION_SERIAL,
  "\n",
  include_str!("close_connection.lua"),
);

/// Удаляет все attachments соединения и закрывает ledger с общим retention.
pub(super) const DISCONNECT: &str =
  concatcp!(LUA_CLOSE_CONNECTION, "\n", include_str!("disconnect.lua"));

/// Очищает соединение истёкшего поколения под node lease reaper-а и cleanup lock.
pub(super) const REAP_CONNECTION: &str = concatcp!(
  LUA_CLOSE_CONNECTION,
  "\n",
  include_str!("check_reap_generation.lua"),
  "\n",
  include_str!("reap_connection.lua"),
);

pub(super) const REAP_GENERATION: &str = concatcp!(
  LUA_CHECK_NODE_LEASE,
  "\n",
  include_str!("check_reap_generation.lua"),
  "\n",
  include_str!("reap_generation.lua"),
);
