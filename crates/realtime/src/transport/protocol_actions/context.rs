use crate::{AttachmentService, ChannelRouter, Connection, OutboundSender, PresenceService};
use event_bus::EventBus;

pub struct SocketContext<'a> {
  pub connection: &'a Connection,
  pub sender: &'a OutboundSender,
  pub router: &'a ChannelRouter,
  pub attachments: &'a AttachmentService,
  pub presence: &'a PresenceService,
  pub event_bus: &'a EventBus,
}
