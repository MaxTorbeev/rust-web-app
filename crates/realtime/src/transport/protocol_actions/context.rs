use std::sync::Arc;
use event_bus::EventBus;
use crate::{ChannelHub, Connection, OutboundSender, PresenceHub};

pub struct SocketContext<'a> {
  pub connection: &'a Connection,
  pub sender: &'a OutboundSender,
  pub channel_hub: &'a Arc<ChannelHub>,
  pub presence_hub: &'a Arc<PresenceHub>,
  pub event_bus: &'a EventBus,
}
