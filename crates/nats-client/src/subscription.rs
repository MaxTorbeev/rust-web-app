use async_nats::jetstream::consumer::pull::{
  Stream as DriverMessageStream,
};

pub struct Subscription {
  messages: DriverMessageStream,
}

impl Subscription {
  pub(crate) fn new(messages: DriverMessageStream) -> Self {
    Self { messages }
  }
}