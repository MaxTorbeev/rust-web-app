use crate::protocol_handlers::SocketContext;
use crate::ProtocolMessage;

pub async fn auth(message: ProtocolMessage, context: &SocketContext<'_>) -> ProtocolMessage {
  match message.auth.as_ref() {
    Some(_auth) => {
      // validate auth.access_tolen
      ProtocolMessage::connected(context.connection)
    }
    None => ProtocolMessage::nack(message.msg_serial)
  }
}