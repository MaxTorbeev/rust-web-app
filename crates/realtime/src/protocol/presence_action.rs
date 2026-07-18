use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[repr(u8)]
pub enum PresenceAction {
  Absent = 0,
  Present = 1,
  Enter = 2,
  Leave = 3,
  Update = 4,
}