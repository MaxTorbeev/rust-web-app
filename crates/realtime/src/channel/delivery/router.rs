use crate::channel::delivery::{BroadcastError, BroadcastOutcome};
use crate::{
  ChannelMode, ConnectionId, OutboundSendError, OutboundSender, PreparedFrame, ProtocolAction,
  ProtocolMessage,
};
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

/// One sender per active WebSocket connection. `ChannelRouter` stores it so
/// broadcasts can enqueue `ProtocolMessage` values without owning the WebSocket
/// itself.
pub type ConnectionSender = OutboundSender;

pub(super) struct LocalAttachment {
  pub(super) sender: ConnectionSender,
  pub(super) modes: Vec<ChannelMode>,
  pub(super) revision: Option<u64>,
  pub(super) pending_revision: u64,
}

#[derive(Default)]
pub struct ChannelState {
  pub(super) channels: HashMap<String, HashMap<ConnectionId, LocalAttachment>>,
  connections: HashMap<ConnectionId, HashSet<String>>,
}

pub struct ChannelRouter {
  pub(super) state: RwLock<ChannelState>,
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
    self
      .register(
        channel,
        connection_id,
        sender,
        ChannelMode::ALL.to_vec(),
        Some(0),
      )
      .await;
  }

  pub(super) async fn register(
    &self,
    channel: &str,
    connection_id: ConnectionId,
    sender: ConnectionSender,
    modes: Vec<ChannelMode>,
    revision: Option<u64>,
  ) {
    let mut state = self.state.write().await;
    let channel = channel.to_string();

    state.channels.entry(channel.clone()).or_default().insert(
      connection_id.clone(),
      LocalAttachment {
        sender,
        modes,
        revision,
        pending_revision: 0,
      },
    );

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
            .filter(|(_, attachment)| {
              attachment.revision.is_some()
                && match message.action {
                  ProtocolAction::Presence => {
                    attachment.modes.contains(&ChannelMode::PresenceSubscribe)
                  }
                  ProtocolAction::Message => attachment.modes.contains(&ChannelMode::Subscribe),
                  _ => true,
                }
            })
            .map(|(connection_id, attachment)| (connection_id.clone(), attachment.sender.clone()))
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
