use event_bus::Event;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebsocketDisconnected {
  pub connection_id: String,
}

impl Event for WebsocketDisconnected {
  const NAME: &'static str = "realtime.websocket_disconnected";
}
