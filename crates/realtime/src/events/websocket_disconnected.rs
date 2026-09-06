use event_bus::Event;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebsocketDisconnected {
  pub connection_id: String,
}

impl Event for WebsocketDisconnected {
  const NAME: &'static str = "realtime.websocket_disconnected";
}
