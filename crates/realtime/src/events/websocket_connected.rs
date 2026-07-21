use serde::{Deserialize, Serialize};
use event_bus::Event;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebsocketConnected {
  pub connection_id: String
}

impl Event for WebsocketConnected {
  const NAME: &'static str = "realtime.websocket_connected";
}