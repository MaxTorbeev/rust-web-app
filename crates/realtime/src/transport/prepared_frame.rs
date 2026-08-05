use axum::extract::ws::Message as WsMessage;
use crate::ProtocolMessage;

/// Готовое к отправке сообщение
#[derive(Clone, Debug)]
pub struct PreparedFrame(WsMessage);

impl TryFrom<&ProtocolMessage> for PreparedFrame {
  type Error = serde_json::Error;

  fn try_from(message: &ProtocolMessage) -> Result<Self, Self::Error> {
    let text = serde_json::to_string(message)?;

    Ok(Self(WsMessage::Text(text.into())))
  }
}

impl PreparedFrame {
  pub fn into_websocket_message(self) -> WsMessage {
    self.0
  }
}