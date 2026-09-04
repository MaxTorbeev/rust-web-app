use support::timestamp::Timestamp;
use crate::{ChannelKey, DetachCommand, ProtocolMessage};
use crate::transport::SocketContext;

pub async fn detach(message: ProtocolMessage, context: &SocketContext<'_>) -> ProtocolMessage {
  let Some(channel) = message.channel.as_deref() else {
    return ProtocolMessage::nack(message.msg_serial);
  };

  let command = DetachCommand {
    channel: ChannelKey::new(
      context.connection.application_id().clone(),
      channel,
    ),
    actor: context.connection.actor(),
    request_time: Timestamp::now(),
  };

  if let Err(error) = context.attachments.detach(command).await {
    tracing::error!(
      %error,
      connection_id = context.connection.id.as_str(),
      %channel,
      "failed to detach connection from channel"
    );

    return ProtocolMessage::nack(message.msg_serial);
  }

  context
    .router
    .detach(channel, &context.connection.id)
    .await;

  ProtocolMessage::detached(&message)
}
