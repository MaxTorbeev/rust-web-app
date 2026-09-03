use serde_repr::{Deserialize_repr, Serialize_repr};

/// Presence action encoded in the Ably wire protocol.
///
/// `Absent` and `Present` describe presence state and synchronization. They are
/// not commands that mutate the authoritative Presence state.
#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, PartialEq, Serialize_repr)]
#[repr(u8)]
pub enum PresenceAction {
  Absent = 0,
  Present = 1,
  Enter = 2,
  Leave = 3,
  Update = 4,
}
