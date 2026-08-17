use serde::{Deserialize, Serialize};
use event_bus::Event;
use crate::{ApplicationId, Message};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelMessagePublished {
  pub application_id: ApplicationId,
  pub channel: String,
  pub messages: Vec<Message>,
}

impl Event for ChannelMessagePublished {
  const NAME: &'static str = "realtime.channel_message_published";
}