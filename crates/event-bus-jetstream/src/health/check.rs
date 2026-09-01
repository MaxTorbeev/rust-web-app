use std::future::{Future, ready};

use support::health::VerifyHealth;
use tokio::sync::watch;

use super::HealthState;

/// Reads the latest lifecycle state reported by the incoming consumer.
#[derive(Clone)]
pub struct HealthCheck {
  receiver: watch::Receiver<HealthState>,
}

impl HealthCheck {
  pub(super) const fn new(receiver: watch::Receiver<HealthState>) -> Self {
    Self { receiver }
  }

  /// Returns the latest lifecycle state without waiting for a transition.
  pub fn state(&self) -> HealthState {
    *self.receiver.borrow()
  }
}

impl VerifyHealth for HealthCheck {
  type Report = HealthState;

  fn verify(&self) -> impl Future<Output = Self::Report> + Send + '_ {
    ready(self.state())
  }
}
