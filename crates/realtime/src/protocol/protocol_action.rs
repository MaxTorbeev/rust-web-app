use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize_repr, Deserialize_repr, Clone)]
#[repr(u8)]
pub enum ProtocolAction {
  Heartbeat = 0,
  Ack = 1,
  Nack = 2,
  Connect = 3,
  Connected = 4,
  Disconnect = 5,
  Disconnected = 6,
  Close = 7,
  Closed = 8,
  Error = 9,
  Attach = 10,
  Attached = 11,
  Presence = 14,
  Message = 15,
  Sync = 16,
  Auth = 17,
}