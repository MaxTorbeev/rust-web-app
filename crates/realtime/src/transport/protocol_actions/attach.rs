use crate::transport::SocketContext;
use crate::{ProtocolFlag, ProtocolMessage};

pub async fn attach(message: ProtocolMessage, context: &SocketContext<'_>) -> Vec<ProtocolMessage> {
  let Some(channel) = message.channel.as_deref() else {
    return vec![ProtocolMessage::nack(message.msg_serial)];
  };

  // TODO(security): WARNING: channel access is not checked against token capability.
  // Authorize the requested channel before attaching the connection.
  context
    .router
    .attach(
      channel,
      context.connection.id.clone(),
      context.sender.clone(),
    )
    .await;

  let presence = context.presence_hub.snapshot(channel).await;

  vec![
    // Отправить оповещение о том что клиент добавлен
    ProtocolMessage::attached(&message, ProtocolFlag::HAS_PRESENCE),
    // Отправить snapshot присутствующих клиентов
    ProtocolMessage::sync(channel, presence),
  ]
}
