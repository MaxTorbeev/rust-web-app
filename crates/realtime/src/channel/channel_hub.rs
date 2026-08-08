use std::collections::{HashMap, HashSet};
use tokio::sync::{RwLock};
use crate::{ConnectionId, OutboundSendError, OutboundSender, PreparedFrame, ProtocolMessage};

/// One sender per active WebSocket connection. ChannelHub stores it so broadcasts
/// can enqueue ProtocolMessage values without owning the WebSocket itself.
pub type ConnectionSender = OutboundSender;

#[derive(Default)]
pub struct ChannelHubState {
  channels: HashMap<String, HashMap<ConnectionId, ConnectionSender>>,
  connections: HashMap<ConnectionId, HashSet<String>>,
}


pub struct ChannelHub {
  /// Map channel names to active WebSocket connections subscribed on this instances
  state: RwLock<ChannelHubState>,
}

impl ChannelHub {
  pub fn new() -> Self {
    Self {
      state: RwLock::new(ChannelHubState::default())
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

  pub async fn detach(&self, channel: &str, connection_id: &ConnectionId) ->bool {
    let mut state = self.state.write().await;

    Self::detach_locked(&mut state, channel, connection_id)
  }

  /// Broadcasts a protocol message to all local connections attached to a channel.
  /// Dead connections are detached from the channel.
  pub async fn broadcast(&self, channel: &str, message: ProtocolMessage) -> usize {
    let targets = {
      let state = self.state.read().await;

      state.channels
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
      return 0;
    }

    let frame = match PreparedFrame::try_from(&message) {
      Ok(frame) => frame,
      Err(error) => {
        tracing::error!(%error, %channel, "failed to prepare broadcast frame");

        return 0;
      }
    };

    let mut sent = 0;
    let mut connections_to_disconnect = Vec::new();

    for (connection_id, sender) in targets {
      match sender.try_enqueue_prepared_frame(frame.clone()) {
        Ok(()) => {
          sent += 1;
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

    for connection_id in connections_to_disconnect {
      self.disconnect(&connection_id).await;
    }

    sent
  }

  /// Removes a local WebSocket connection from all attached channels.
  /// Returns the channels that were affected, so presence cleanup can emit leave events.
  pub async fn disconnect(&self, connection_id: &ConnectionId) ->Vec<String> {
    let mut state = self.state.write().await;

    let channels = state
      .connections
      .remove(connection_id)
      .unwrap_or_default();

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

    state.channels.get(channel).is_some_and(|connections| {
      connections.contains_key(connection_id)
    })
  }

  fn detach_locked(state: &mut ChannelHubState, channel: &str, connection_id: &ConnectionId) -> bool {
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Duration;
  use axum::extract::ws::Message as WebSocketMessage;
  use tokio::sync::mpsc;
  use tokio::time::timeout;
  use crate::ProtocolAction;
  use crate::transport::{shutdown_channel, ShutdownListener};

  const TEST_TIMEOUT: Duration = Duration::from_secs(1);
  const CHANNEL_A: &str = "channel-a";
  const CHANNEL_B: &str = "channel-b";

  fn test_connection(
    capacity: usize,
  ) -> (
    OutboundSender,
    mpsc::Receiver<PreparedFrame>,
    ShutdownListener,
  ) {
    let (shutdown_trigger, shutdown_listener) = shutdown_channel();
    let (queue_sender, queue_receiver) = mpsc::channel(capacity);

    (
      OutboundSender::new(queue_sender, shutdown_trigger),
      queue_receiver,
      shutdown_listener,
    )
  }

  fn assert_heartbeat_frame(frame: PreparedFrame) {
    let WebSocketMessage::Text(text) = frame.into_websocket_message() else {
      panic!("expected a text WebSocket frame");
    };

    let message = serde_json::from_str::<ProtocolMessage>(text.as_str())
      .expect("broadcast frame must contain a protocol message");

    assert!(matches!(message.action, ProtocolAction::Heartbeat));
  }

  #[tokio::test(flavor = "current_thread")]
  async fn full_recipient_does_not_block_healthy_recipient() {
    let hub = ChannelHub::new();
    let slow_connection_id = ConnectionId::generate();
    let healthy_connection_id = ConnectionId::generate();

    let (slow_sender, _slow_receiver, mut slow_shutdown) = test_connection(1);
    let (healthy_sender, mut healthy_receiver, _healthy_shutdown) = test_connection(1);

    // Only the slow connection starts with a full outbound queue.
    slow_sender
      .try_enqueue_protocol_message(&ProtocolMessage::heartbeat())
      .expect("the first frame must fill the slow queue");

    // The second subscription proves that overflow disconnects the whole
    // connection, not only its subscription to the broadcast channel.
    hub.attach(CHANNEL_A, slow_connection_id.clone(), slow_sender.clone()).await;
    hub.attach(CHANNEL_B, slow_connection_id.clone(), slow_sender).await;
    hub.attach(CHANNEL_A, healthy_connection_id.clone(), healthy_sender).await;

    let sent = timeout(
      TEST_TIMEOUT,
      hub.broadcast(CHANNEL_A, ProtocolMessage::heartbeat()),
    )
      .await
      .expect("a full recipient must not block broadcast");

    assert_eq!(sent, 1, "only the healthy queue must accept the frame");

    let healthy_frame = timeout(TEST_TIMEOUT, healthy_receiver.recv())
      .await
      .expect("healthy recipient must receive without delay")
      .expect("healthy queue must remain open");

    assert_heartbeat_frame(healthy_frame);

    assert!(
      timeout(TEST_TIMEOUT, slow_shutdown.requested())
        .await
        .expect("slow connection must receive shutdown"),
    );

    assert!(!hub.is_attached(CHANNEL_A, &slow_connection_id).await);
    assert!(!hub.is_attached(CHANNEL_B, &slow_connection_id).await);
    assert!(hub.is_attached(CHANNEL_A, &healthy_connection_id).await);
  }

  #[tokio::test(flavor = "current_thread")]
  async fn closed_recipient_does_not_block_healthy_recipient() {
    let hub = ChannelHub::new();
    let closed_connection_id = ConnectionId::generate();
    let healthy_connection_id = ConnectionId::generate();

    let (closed_sender, closed_receiver, mut closed_shutdown) = test_connection(1);
    let (healthy_sender, mut healthy_receiver, _healthy_shutdown) = test_connection(1);

    // A dropped receiver models a WebSocket writer that has already stopped.
    drop(closed_receiver);

    hub.attach(CHANNEL_A, closed_connection_id.clone(), closed_sender.clone()).await;
    hub.attach(CHANNEL_B, closed_connection_id.clone(), closed_sender).await;
    hub.attach(CHANNEL_A, healthy_connection_id.clone(), healthy_sender).await;

    let sent = timeout(
      TEST_TIMEOUT,
      hub.broadcast(CHANNEL_A, ProtocolMessage::heartbeat()),
    )
      .await
      .expect("a closed recipient must not block broadcast");

    assert_eq!(sent, 1, "only the healthy queue must accept the frame");

    let healthy_frame = timeout(TEST_TIMEOUT, healthy_receiver.recv())
      .await
      .expect("healthy recipient must receive without delay")
      .expect("healthy queue must remain open");

    assert_heartbeat_frame(healthy_frame);

    assert!(
      timeout(TEST_TIMEOUT, closed_shutdown.requested())
        .await
        .expect("closed connection must receive shutdown"),
    );

    assert!(!hub.is_attached(CHANNEL_A, &closed_connection_id).await);
    assert!(!hub.is_attached(CHANNEL_B, &closed_connection_id).await);
    assert!(hub.is_attached(CHANNEL_A, &healthy_connection_id).await);
  }
}
