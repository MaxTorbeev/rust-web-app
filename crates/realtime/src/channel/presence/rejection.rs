use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PresenceRejection {
  NotAttached,
  UnidentifiedConnection,
  ClientIdNotAllowed { client_id: String },
  InvalidMemberState,
  ConflictingReplay,
}
