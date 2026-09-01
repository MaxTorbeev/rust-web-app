use support::health::HealthReport;

use crate::TopologyError;

/// Current result of a read-only JetStream topology verification.
#[derive(Debug)]
pub enum HealthState {
  Up,
  Down(TopologyError),
}

impl HealthReport for HealthState {
  fn is_healthy(&self) -> bool {
    matches!(self, Self::Up)
  }
}
