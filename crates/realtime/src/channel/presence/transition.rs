use crate::CommittedPresenceEvent;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommittedTransition {
  /// Новая ревизия Presence.
  /// None, если изменился только attachment/Occupancy.
  pub presence_revision: Option<u64>,

  /// Версия полного Occupancy snapshot.
  pub occupancy_version: u64,

  /// Canonical event, созданный этим commit.
  pub event: Option<CommittedPresenceEvent>,
}
