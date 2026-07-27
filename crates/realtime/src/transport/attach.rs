use crate::protocol_handlers::SocketContext;
use crate::ProtocolMessage;

pub async fn attach(message: ProtocolMessage, context: &SocketContext<'_>) -> ProtocolMessage {
  match message.channel.as_deref() {
    Some(channel) => {
      context.channel_hub.attach(channel, context.connection.id.clone(), context.sender.clone()).await;

      ProtocolMessage::attached(&message)
    },
    None => ProtocolMessage::nack(message.msg_serial)
  }
}