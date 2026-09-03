use crate::{CommittedTransition, PresenceDeliveryError};
use std::{future::Future, pin::Pin};

pub type PresenceDeliveryFuture<'a> = Pin<Box<dyn Future<Output = Result<(), PresenceDeliveryError>> + Send + 'a>>;

pub trait PresenceCommitDelivery: Send + Sync {
  fn after_commit<'a>(&'a self, transition: &'a CommittedTransition) -> PresenceDeliveryFuture<'a>;
}