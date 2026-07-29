use std::collections::{HashMap, HashSet};
use tokio::sync::{RwLock};
use support::timestamp::Timestamp;
use crate::{Connection, ConnectionId, PresenceAction, PresenceMessage};

#[derive(Default)]
struct PresenceHubState {
  members: HashMap<String, HashMap<ConnectionId, PresenceMessage>>,
  connections: HashMap<ConnectionId, HashSet<String>>,
}

pub struct PresenceHub {
  state: RwLock<PresenceHubState>,
}

impl PresenceHub {
  pub fn new() -> Self {
    Self {
      state: RwLock::new(PresenceHubState::default()),
    }
  }

  pub async fn enter(&self, channel: &str, connection: &Connection, presence: PresenceMessage) -> PresenceMessage {
    let mut state = self.state.write().await;
    let channel = channel.to_string();
    let presence = Self::server_presence(connection, presence, PresenceAction::Enter);

    state.members
      .entry(channel.clone())
      .or_default()
      .insert(connection.id.clone(), presence.clone());

    state
      .connections
      .entry(connection.id.clone())
      .or_default()
      .insert(channel);

    presence
  }

  pub async fn update(
    &self,
    channel: &str,
    connection: &Connection,
    presence: PresenceMessage
  ) -> Option<PresenceMessage> {
    let mut state = self.state.write().await;
    let update = Self::server_presence(connection, presence, PresenceAction::Update);

    let member = state.members
      .get_mut(channel)?
      .get_mut(&connection.id)?;

    *member = update.clone();

    Some(update)
  }

  pub async fn leave(
    &self,
    channel: &str,
    connection_id: &ConnectionId,
  ) -> Option<PresenceMessage> {
    let mut state = self.state.write().await;

    Self::leave_locked(&mut state, channel, connection_id)
  }

  pub async fn members(&self, channel: &str) -> Vec<PresenceMessage> {
    let state = self.state.read().await;

    state.members.get(channel).map(|members| {
      members.values().cloned().collect()
    }).unwrap_or_default()
  }

  pub async fn disconnect(&self, connection_id: &ConnectionId) -> Vec<(String, PresenceMessage)> {
    let mut state = self.state.write().await;

    let channels = state
      .connections
      .get(connection_id)
      .cloned()
      .unwrap_or_default();

    let mut leaves = Vec::new();

    for channel in channels {
      if let Some(presence) = Self::leave_locked(&mut state, &channel, connection_id) {
        leaves.push((channel, presence));
      }
    }

    leaves
  }

  fn leave_locked(
    state: &mut PresenceHubState,
    channel: &str,
    connection_id: &ConnectionId
  ) -> Option<PresenceMessage> {
    let mut should_remove_channel = false;

    let presence = if let Some(members) = state.members.get_mut(channel) {
      let presence = members.remove(connection_id);

      should_remove_channel = members.is_empty();

      presence
    } else {
      None
    };

    if should_remove_channel {
      state.members.remove(channel);
    }

    if presence.is_some() {
      let mut should_remove_connection = false;

      if let Some(channels) = state.connections.get_mut(connection_id) {
        channels.remove(channel);

        should_remove_connection = channels.is_empty();
      }

      if should_remove_connection {
        state.connections.remove(connection_id);
      }
    }

    presence.map(Self::leave_presence)
  }


  fn server_presence(
    connection: &Connection,
    mut presence: PresenceMessage,
    action: PresenceAction
  ) -> PresenceMessage {
    presence.action = action;
    presence.id = Some(uuid::Uuid::new_v4().to_string());
    presence.client_id = connection.client_id().map(str::to_owned);
    presence.connection_id = Some(connection.id.as_str().to_string());
    presence.timestamp = Some(Timestamp::now().to_string());

    presence
  }

  fn leave_presence(mut presence: PresenceMessage) -> PresenceMessage {
    presence.action = PresenceAction::Leave;
    presence.id = Some(uuid::Uuid::new_v4().to_string());
    presence.timestamp = Some(Timestamp::now().to_string());

    presence
  }
}