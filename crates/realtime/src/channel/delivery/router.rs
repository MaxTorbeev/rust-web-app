use crate::channel::delivery::{BroadcastError, BroadcastOutcome};
use crate::{ConnectionId, OutboundSendError, OutboundSender, PreparedFrame, ProtocolMessage};
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

/// One sender per active WebSocket connection. ChannelHub stores it so broadcasts
/// can enqueue ProtocolMessage values without owning the WebSocket itself.
pub type ConnectionSender = OutboundSender;

#[derive(Default)]
pub struct ChannelState {
  channels: HashMap<String, HashMap<ConnectionId, ConnectionSender>>,
  connections: HashMap<ConnectionId, HashSet<String>>,
}

pub struct ChannelRouter {
  state: RwLock<ChannelState>,
}

impl ChannelRouter {
  pub fn new() -> Self {
    Self {
      state: RwLock::new(ChannelState::default()),
    }
  }

  /// Attaches a local WebSocket connection
  /// to a channel and keeps broadcast/disconnect indexes in sync.
  pub async fn attach(&self, channel: &str, connection_id: ConnectionId, sender: ConnectionSender) {
    let mut state = self.state.write().await;
    let channel = channel.to_string();

    state
      .channels
      .entry(channel.clone())
      .or_default()
      .insert(connection_id.clone(), sender);

    state
      .connections
      .entry(connection_id)
      .or_default()
      .insert(channel);
  }

  pub async fn detach(&self, channel: &str, connection_id: &ConnectionId) -> bool {
    let mut state = self.state.write().await;

    Self::detach_locked(&mut state, channel, connection_id)
  }

  /// Broadcasts a protocol message to all local connections attached to a channel.
  /// Dead connections are detached from the channel.
  pub async fn broadcast(
    &self,
    channel: &str,
    message: ProtocolMessage,
  ) -> Result<BroadcastOutcome, BroadcastError> {
    let targets = {
      let state = self.state.read().await;

      state
        .channels
        .get(channel)
        .map(|connections| {
          connections
            .iter()
            .map(|(connection_id, sender)| (connection_id.clone(), sender.clone()))
            .collect::<Vec<_>>()
        })
        .unwrap_or_default()
    };

    if targets.is_empty() {
      return Ok(BroadcastOutcome::default());
    }

    let frame = PreparedFrame::try_from(&message)?;

    let mut enqueued = 0;
    let mut connections_to_disconnect = Vec::new();

    for (connection_id, sender) in targets {
      match sender.try_enqueue_prepared_frame(frame.clone()) {
        Ok(()) => {
          enqueued += 1;
        }

        Err(OutboundSendError::QueueFull) => {
          tracing::warn!(
            connection_id = connection_id.as_str(),
            %channel,
            "disconnecting slow consumer"
          );
          sender.request_shutdown();
          connections_to_disconnect.push(connection_id);
        }

        Err(OutboundSendError::QueueClosed) => {
          sender.request_shutdown();
          connections_to_disconnect.push(connection_id);
        }

        Err(OutboundSendError::Serialization(_)) => {
          unreachable!("prepared frame is already serialized");
        }
      }
    }

    let disconnected = connections_to_disconnect.len();

    for connection_id in connections_to_disconnect {
      self.disconnect(&connection_id).await;
    }

    Ok(BroadcastOutcome {
      enqueued,
      disconnected,
    })
  }

  /// Removes a local WebSocket connection from all attached channels.
  /// Returns the channels that were affected, so presence cleanup can emit leave events.
  pub async fn disconnect(&self, connection_id: &ConnectionId) -> Vec<String> {
    let mut state = self.state.write().await;

    let channels = state.connections.remove(connection_id).unwrap_or_default();

    for channel in &channels {
      Self::detach_locked(&mut state, channel, connection_id);
    }

    let mut channels = channels.into_iter().collect::<Vec<_>>();

    channels.sort();

    channels
  }

  /// Checks whether a local WebSocket connection is attached to a channel.
  pub async fn is_attached(&self, channel: &str, connection_id: &ConnectionId) -> bool {
    let state = self.state.read().await;

    state
      .channels
      .get(channel)
      .is_some_and(|connections| connections.contains_key(connection_id))
  }

  fn detach_locked(state: &mut ChannelState, channel: &str, connection_id: &ConnectionId) -> bool {
    let mut should_remove_channel = false;

    let removed = if let Some(connections) = state.channels.get_mut(channel) {
      let removed = connections.remove(connection_id).is_some();

      should_remove_channel = connections.is_empty();

      removed
    } else {
      false
    };

    if should_remove_channel {
      state.channels.remove(channel);
    }

    if removed {
      let mut should_remove_connection = false;

      if let Some(channels) = state.connections.get_mut(connection_id) {
        channels.remove(channel);

        should_remove_connection = channels.is_empty();
      }

      if should_remove_connection {
        state.connections.remove(connection_id);
      }
    }

    removed
  }
}
