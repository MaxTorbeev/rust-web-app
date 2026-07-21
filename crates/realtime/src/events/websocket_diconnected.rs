use serde::{Deserialize, Serialize};
use event_bus::Event;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebsocketDisconnected {
  pub connection_id: String
}

impl Event for WebsocketDisconnected {
  const NAME: &'static str = "realtime.websocket_diconnected";
}