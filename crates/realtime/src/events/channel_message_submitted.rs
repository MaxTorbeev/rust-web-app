use serde::{Deserialize, Serialize};
use event_bus::{DeliveryClass, Event};
use crate::{ApplicationId, Message};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelMessageSubmitted {
  pub application_id: ApplicationId,
  pub channel: String,
  pub messages: Vec<Message>,
}

impl Event for ChannelMessageSubmitted {
  const NAME: &'static str = "realtime.channel_message_submitted";
  const DELIVERY: DeliveryClass = DeliveryClass::AllNodes;
}
