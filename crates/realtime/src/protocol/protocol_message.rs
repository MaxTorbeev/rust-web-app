use serde::{Deserialize, Serialize};
use crate::PresenceMessage;

#[derive(Serialize, Deserialize)]
pub struct ProtocolMessage {

  #[serde(skip_serializing_if="Option::is_none")]
  pub presence: Option<Vec<PresenceMessage>>
}