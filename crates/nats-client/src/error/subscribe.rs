use async_nats::jetstream::context::PublishError as DriverPublishError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("failed to subscribe JetStream message: {source}")]
pub struct SubscribeError {

}
