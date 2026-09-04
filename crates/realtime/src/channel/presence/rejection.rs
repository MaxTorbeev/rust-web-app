#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PresenceRejection {
  NotAttached,
  UnidentifiedConnection,
  ClientIdNotAllowed {
    client_id: String,
  },
  InvalidMemberState,
  ConflictingReplay,
}