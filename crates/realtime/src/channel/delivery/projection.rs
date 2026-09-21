use crate::{
  ChannelCommitDeliveryError, ChannelMode, ChannelRouter, ConnectionId, OutboundSender,
  PreparedFrame, PresenceChannelChanged, PresenceMessage, PresenceSnapshot, PresenceStore,
  ProtocolMessage,
};

impl ChannelRouter {
  /// Pending хранит только максимальную полученную revision, без очереди payload.
  pub(crate) async fn begin_attach(
    &self,
    channel: &str,
    connection: ConnectionId,
    sender: OutboundSender,
    modes: Vec<ChannelMode>,
  ) {
    self
      .register(channel, connection, sender, modes, None)
      .await;
  }

  /// Ставит ATTACHED/SYNC в очередь под тем же lock, под которым проходят deltas.
  /// false: во время чтения пришла более новая revision, snapshot нужно перечитать.
  pub(crate) async fn finish_attach(
    &self,
    channel: &str,
    connection: &ConnectionId,
    attached: &ProtocolMessage,
    snapshot: &PresenceSnapshot,
  ) -> Result<bool, crate::BroadcastError> {
    let mut state = self.state.write().await;
    let Some(local) = state
      .channels
      .get_mut(channel)
      .and_then(|c| c.get_mut(connection))
    else {
      return Ok(true);
    };
    let presence = local.modes.contains(&ChannelMode::PresenceSubscribe);
    if presence && local.pending_revision > snapshot.presence_revision {
      return Ok(false);
    }
    let mut attached = attached.clone();
    attached.channel_serial = Some(format!(
      "{}:{}",
      snapshot.presence_revision, snapshot.occupancy_version
    ));
    let attached = PreparedFrame::try_from(&attached)?;
    let sync = if presence {
      Some(PreparedFrame::try_from(&ProtocolMessage::sync(
        channel,
        snapshot.members.iter().map(PresenceMessage::from).collect(),
      ))?)
    } else {
      None
    };
    if local.sender.try_enqueue_prepared_frame(attached).is_err()
      || sync.is_some_and(|frame| local.sender.try_enqueue_prepared_frame(frame).is_err())
    {
      local.sender.request_shutdown();
    }
    local.revision = Some(snapshot.presence_revision);
    Ok(true)
  }

  /// Revision устраняет повторы транспорта; gap восстанавливается authoritative snapshot.
  pub(crate) async fn project_presence(
    &self,
    change: &PresenceChannelChanged,
    store: &dyn PresenceStore,
  ) -> Result<(), ChannelCommitDeliveryError> {
    let Some(revision) = change.presence_revision else {
      return Ok(());
    };
    let channel = &change.channel.channel;
    let mut snapshot: Option<PresenceSnapshot> = None;
    loop {
      let mut state = self.state.write().await;
      let Some(locals) = state.channels.get_mut(channel) else {
        return Ok(());
      };
      for local in locals.values_mut().filter(|local| local.revision.is_none()) {
        local.pending_revision = local.pending_revision.max(revision);
      }
      let gap = locals.values().any(|local| {
        local.modes.contains(&ChannelMode::PresenceSubscribe)
          && local
            .revision
            .is_some_and(|current| revision > current.saturating_add(1))
      });
      if gap && snapshot.is_none() {
        drop(state);
        let current = store.snapshot(change.channel.clone()).await.map_err(|e| {
          ChannelCommitDeliveryError::Projection {
            message: e.to_string(),
          }
        })?;
        if current.presence_revision < revision {
          return Err(ChannelCommitDeliveryError::Projection {
            message: "snapshot is behind committed Presence event".into(),
          });
        }
        snapshot = Some(current);
        continue;
      }
      let version = snapshot
        .as_ref()
        .map_or(revision, |snapshot| snapshot.presence_revision);
      if !locals.values().any(|local| {
        local.modes.contains(&ChannelMode::PresenceSubscribe)
          && local.revision.is_some_and(|current| current < version)
      }) {
        return Ok(());
      }
      let (version, frame) = match &snapshot {
        Some(snapshot) => (
          snapshot.presence_revision,
          ProtocolMessage::sync(
            channel,
            snapshot.members.iter().map(PresenceMessage::from).collect(),
          ),
        ),
        None => (
          revision,
          ProtocolMessage::presence(
            channel,
            change
              .member_changes
              .iter()
              .map(PresenceMessage::from)
              .collect(),
          ),
        ),
      };
      let frame = PreparedFrame::try_from(&frame).map_err(crate::BroadcastError::from)?;
      for local in locals.values_mut() {
        if local.modes.contains(&ChannelMode::PresenceSubscribe)
          && local.revision.is_some_and(|current| current < version)
        {
          if local
            .sender
            .try_enqueue_prepared_frame(frame.clone())
            .is_err()
          {
            local.sender.request_shutdown();
          }
          local.revision = Some(version);
        }
      }
      return Ok(());
    }
  }

  pub async fn shutdown(&self) {
    let state = self.state.read().await;
    for locals in state.channels.values() {
      for local in locals.values() {
        local.sender.request_shutdown();
      }
    }
  }
}
