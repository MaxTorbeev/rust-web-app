use serde_repr::{Deserialize_repr, Serialize_repr};

/// Presence action encoded in the realtime protocol.
///
/// `Absent` and `Present` describe snapshot state and are not valid mutation
/// commands. Store mutations use [`PresenceMutationAction`] instead.
#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, PartialEq, Serialize_repr)]
#[repr(u8)]
pub enum PresenceAction {
  Absent = 0,
  Present = 1,
  Enter = 2,
  Leave = 3,
  Update = 4,
}

/// A client action that may mutate authoritative Presence state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresenceMutationAction {
  Enter,
  Leave,
  Update,
}

impl TryFrom<PresenceAction> for PresenceMutationAction {
  type Error = PresenceAction;

  fn try_from(action: PresenceAction) -> Result<Self, Self::Error> {
    match action {
      PresenceAction::Enter => Ok(Self::Enter),
      PresenceAction::Leave => Ok(Self::Leave),
      PresenceAction::Update => Ok(Self::Update),
      PresenceAction::Absent | PresenceAction::Present => Err(action),
    }
  }
}

impl From<PresenceMutationAction> for PresenceAction {
  fn from(action: PresenceMutationAction) -> Self {
    match action {
      PresenceMutationAction::Enter => Self::Enter,
      PresenceMutationAction::Leave => Self::Leave,
      PresenceMutationAction::Update => Self::Update,
    }
  }
}