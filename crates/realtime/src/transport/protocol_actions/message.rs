use crate::{ChannelMessageSubmitted, ProtocolMessage};
use crate::transport::SocketContext;

pub async fn message(message: ProtocolMessage, context: &SocketContext<'_>) -> ProtocolMessage {
  match message.channel.as_deref() {
    Some(channel) => {
      // TODO(security): WARNING: publishing is not checked against token capability.
      // Require `publish` permission for this channel before broadcasting.
      let result = context
        .event_bus
        .publish(ChannelMessageSubmitted {
          application_id: context.connection.application_id().clone(),
          channel: channel.to_owned(),
          messages: message.messages.clone().unwrap_or_default(),
        })
        .await;

      match result {
        Ok(_) => ProtocolMessage::ack(&message),
        Err(error) => {
          tracing::error!(%error, %channel, "failed to publish channel message event");

          ProtocolMessage::nack(message.msg_serial)
        }
      }
    },
    None => ProtocolMessage::nack(message.msg_serial)
  }
}
