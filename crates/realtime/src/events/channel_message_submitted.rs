use crate::{ApplicationId, Message};
use event_bus::{DeliveryClass, Event};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelMessageSubmitted {
  pub application_id: ApplicationId,
  pub channel: String,
  pub messages: Vec<Message>,
}

impl Event for ChannelMessageSubmitted {
  const NAME: &'static str = "realtime.channel_message_submitted";
  const DELIVERY: DeliveryClass = DeliveryClass::AllNodes;
}
