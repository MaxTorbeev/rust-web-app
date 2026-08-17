use crate::ProtocolMessage;
use crate::transport::SocketContext;

pub async fn message(message: ProtocolMessage, context: &SocketContext<'_>) -> ProtocolMessage {
  match message.channel.as_deref() {
    Some(channel) => {
      // TODO(security): WARNING: publishing is not checked against token capability.
      // Require `publish` permission for this channel before broadcasting.
      let result = context
        .channel_hub
        .broadcast(channel, message.clone())
        .await;

      match result {
        Ok(_) => ProtocolMessage::ack(&message),
        Err(error) => {
          tracing::error!(%error, %channel, "failed to broadcast channel message");

          ProtocolMessage::nack(message.msg_serial)
        }
      }
    },
    None => ProtocolMessage::nack(message.msg_serial)
  }
}
