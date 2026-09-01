use support::health::HealthReport;

use crate::TopologyError;

use super::HealthState;

#[test]
fn reports_up_as_healthy() {
  assert!(HealthState::Up.is_healthy());
}

#[test]
fn reports_down_as_unhealthy() {
  let error = TopologyError::consumer_stream_mismatch("realtime-node-1", "OTHER_EVENTS", "EVENTS");

  assert!(!HealthState::Down(error).is_healthy());
}
