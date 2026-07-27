use crate::transport::attach::attach;
use crate::{
  ChannelHub, 
  Connection, 
  PresenceHub,
  ProtocolAction, 
  ProtocolMessage, 
  auth, 
  detach, 
  message as message_handler,
  presence
};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub struct SocketContext<'a> {
    pub connection: &'a Connection,
    pub sender: &'a UnboundedSender<ProtocolMessage>,
    pub channel_hub: &'a Arc<ChannelHub>,
    pub presence_hub: &'a Arc<PresenceHub>,
}

pub struct ProtocolHandleResult {
    pub response: Option<ProtocolMessage>,
    pub disconnect: bool,
}

pub async fn handle_protocol_message(
    message: ProtocolMessage,
    context: &SocketContext<'_>,
) -> ProtocolHandleResult {
    let mut disconnect = false;

    let response = match message.action {
        ProtocolAction::Connect => Some(ProtocolMessage::connected(context.connection)),
        ProtocolAction::Auth => Some(auth(message, context).await),
        ProtocolAction::Disconnect => {
            disconnect = true;

            Some(ProtocolMessage::disconnected())
        }
        ProtocolAction::Attach =>Some(attach(message, context).await),
        ProtocolAction::Presence => Some(presence(message, context).await),
        ProtocolAction::Message => Some(message_handler(message, context).await),
        ProtocolAction::Heartbeat => Some(ProtocolMessage::heartbeat()),
        ProtocolAction::Detach => Some(detach(message, context).await),
        _ => None,
    };

    ProtocolHandleResult {
        response,
        disconnect,
    }
}
