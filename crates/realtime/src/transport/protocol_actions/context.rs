use crate::{ChannelHub, Connection, OutboundSender, PresenceHub};
use event_bus::EventBus;
use std::sync::Arc;

pub struct SocketContext<'a> {
  pub connection: &'a Connection,
  pub sender: &'a OutboundSender,
  pub channel_hub: &'a Arc<ChannelHub>,
  pub presence_hub: &'a Arc<PresenceHub>,
  pub event_bus: &'a EventBus,
}
