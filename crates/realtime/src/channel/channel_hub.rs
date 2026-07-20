use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use crate::{ConnectionId, ProtocolMessage};

// One sender per active WebSocket connection. ChannelHub stores it so broadcasts
// can enqueue ProtocolMessage values without owning the WebSocket itself.
pub type ConnectionSender = mpsc::UnboundedSender<ProtocolMessage>;

pub struct ChannelHub {
  // Map channel names to active WebSocket connections subscribed on this instances
  channels: RwLock<HashMap<String, HashMap<ConnectionId, ConnectionSender>>>,
}

impl ChannelHub {
  pub fn new() -> Self {
    Self {
      channels: RwLock::new(HashMap::new()),
    }
  }
  pub async fn attach(&self, channel: &str, connection_id: ConnectionId, sender: ConnectionSender) {
    let mut channels = self.channels.write().await;

    channels
      .entry(channel.to_string())
      .or_default()
      .insert(connection_id, sender);
  }
  pub async fn detach(&self, channel: &str, connection_id: &ConnectionId) {
    let mut channels = self.channels.write().await;

    if let Some(connections) = channels.get_mut(channel) {
      connections.remove(connection_id);

      if connections.is_empty() {
        channels.remove(channel);
      }
    }
  }
  pub async fn broadcast(&self, channel: &str, message: ProtocolMessage) {
    let senders = {
      let channels = self.channels.read().await;

      channels
        .get(channel)
        .map(|connections| connections.values().cloned().collect::<Vec<_>>())
        .unwrap_or_default()
    };

    for sender in senders {
      let _ = sender.send(message.clone());
    }
  }
  pub async fn disconnect(&self, connection_id: &ConnectionId) {
    let mut channels = self.channels.write().await;

    channels.retain(|_, connections| {
      connections.remove(connection_id);

      !connections.is_empty()
    });
  }
}