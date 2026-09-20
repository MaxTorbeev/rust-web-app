//! Lua-скрипты Redis-адаптера состояния каналов.

use const_format::concatcp;
use redis_lease::{LUA_ACQUIRE_LEASE, LUA_RENEW_LEASE};

/// Захват node lease вместе с регистрацией поколения и его deadline.
pub(super) const CLAIM_NODE: &str =
  concatcp!(LUA_ACQUIRE_LEASE, "\n", include_str!("claim_node.lua"),);

/// Продление node lease вместе с deadline его поколения.
pub(super) const RENEW_NODE: &str =
  concatcp!(LUA_RENEW_LEASE, "\n", include_str!("renew_node.lua"),);
