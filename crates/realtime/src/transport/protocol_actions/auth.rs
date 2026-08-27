use crate::ProtocolMessage;
use crate::transport::SocketContext;

pub async fn auth(message: ProtocolMessage, context: &SocketContext<'_>) -> ProtocolMessage {
  match message.auth.as_ref() {
    Some(_auth) => {
      // TODO(security): WARNING: AUTH currently accepts any payload and keeps the old authorization.
      // Verify the new access token, replace connection authorization and reject failed renewal.
      ProtocolMessage::connected(context.connection)
    }
    None => ProtocolMessage::nack(message.msg_serial),
  }
}
