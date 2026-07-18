use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ProtocolAction {
  Heartbeat = 0,
  Ack = 1,
  Nack = 2,
  Connect = 3,
  Connected = 4,
  Attach = 10,
  Attached = 11,
  Message = 15,
}