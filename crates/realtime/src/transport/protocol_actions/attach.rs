use crate::transport::SocketContext;
use crate::{
  AttachCommand, AttachmentTracking, ChannelKey, ChannelMode, OCCUPANCY_CAPABILITY_OPERATION,
  OCCUPANCY_PARAM, OccupancySubscription, PresenceMessage, ProtocolFlag, ProtocolMessage,
  resolve_effective_modes,
};
use std::collections::BTreeMap;
use support::fresh_uuid;
use support::timestamp::Timestamp;

pub async fn attach(message: ProtocolMessage, context: &SocketContext<'_>) -> Vec<ProtocolMessage> {
  let Some(channel) = message.channel.as_deref() else {
    return vec![ProtocolMessage::nack(message.msg_serial)];
  };

  let connection = context.connection;
  let capability = &connection.authorization().capability;

  // Effective modes — пересечение запрошенных режимов и capability токена.
  // Пустое пересечение означает отсутствие доступа к каналу.
  let effective_modes =
    resolve_effective_modes(capability, channel, ProtocolFlag::from_wire(message.flags));

  if effective_modes.is_empty() {
    tracing::warn!(
      connection_id = connection.id.as_str(),
      %channel,
      "attach denied: token capability does not cover channel"
    );

    return vec![ProtocolMessage::nack(message.msg_serial)];
  }

  // Подписка на Occupancy запрашивается через params и требует `channel-metadata`.
  let occupancy = match message.param(OCCUPANCY_PARAM) {
    None => None,
    Some(value) => {
      if !capability.allows(channel, OCCUPANCY_CAPABILITY_OPERATION) {
        tracing::warn!(
          connection_id = connection.id.as_str(),
          %channel,
          "attach denied: occupancy requires channel-metadata capability"
        );

        return vec![ProtocolMessage::nack(message.msg_serial)];
      }

      match OccupancySubscription::parse(value) {
        Ok(subscription) => Some(subscription),
        Err(error) => {
          tracing::warn!(%error, connection_id = connection.id.as_str(), %channel, "invalid attach params");

          return vec![ProtocolMessage::nack(message.msg_serial)];
        }
      }
    }
  };

  let mut attached_params = BTreeMap::new();

  if let Some(subscription) = &occupancy {
    attached_params.insert(OCCUPANCY_PARAM.to_owned(), subscription.to_wire_value());
  }

  let attached_flags = ProtocolFlag::HAS_PRESENCE | ChannelMode::to_flags(&effective_modes);

  let command = AttachCommand {
    channel: ChannelKey::new(connection.application_id().clone(), channel),
    actor: connection.actor(),
    accounting: AttachmentTracking::Individual,
    effective_modes,
    occupancy,
    request_time: Timestamp::now(),
    event_id: fresh_uuid(),
  };

  // Сначала фиксируем attachment в хранилище: снимок Presence должен
  // соответствовать состоянию, в котором соединение уже учтено.
  let outcome = match context.attachments.attach(command).await {
    Ok(outcome) => outcome,
    Err(error) => {
      tracing::error!(
        %error,
        connection_id = connection.id.as_str(),
        %channel,
        "failed to attach connection to channel"
      );

      return vec![ProtocolMessage::nack(message.msg_serial)];
    }
  };

  // Затем регистрируем соединение в локальной доставке.
  context
    .router
    .attach(channel, connection.id.clone(), context.sender.clone())
    .await;

  let presence = outcome
    .snapshot
    .members
    .iter()
    .map(PresenceMessage::from)
    .collect();

  // TODO(occupancy): отправлять initial `[meta]occupancy`, если подписка запрошена.
  vec![
    // Отправить оповещение о том что клиент добавлен
    ProtocolMessage::attached(&message, attached_flags, attached_params),
    // Отправить snapshot присутствующих клиентов
    ProtocolMessage::sync(channel, presence),
  ]
}
