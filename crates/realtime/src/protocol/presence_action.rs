use serde::{Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize_repr, Deserialize_repr, Clone)]
#[repr(u8)]
pub enum PresenceAction {
  Absent = 0,
  Present = 1,
  Enter = 2,
  Leave = 3,
  Update = 4,
}