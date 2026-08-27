use event_bus::Event;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebsocketConnected {
  pub connection_id: String,
}

impl Event for WebsocketConnected {
  const NAME: &'static str = "realtime.websocket_connected";
}
