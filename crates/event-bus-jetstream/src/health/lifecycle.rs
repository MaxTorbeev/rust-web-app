use std::task::Poll;

use tokio::sync::watch;

use super::{HealthCheck, HealthState};

pub(crate) struct HealthLifecycle {
  sender: watch::Sender<HealthState>,
}

impl HealthLifecycle {
  pub(crate) fn new() -> Self {
    let (sender, _) = watch::channel(HealthState::Starting);

    Self { sender }
  }

  pub(crate) fn health_check(&self) -> HealthCheck {
    HealthCheck::new(self.sender.subscribe())
  }

  pub(crate) fn observe_first_poll<T, E>(&self, poll: &Poll<Option<Result<T, E>>>) {
    if *self.sender.borrow() != HealthState::Starting {
      return;
    }

    let next = match poll {
      Poll::Pending | Poll::Ready(Some(Ok(_))) => HealthState::Running,
      Poll::Ready(Some(Err(_))) | Poll::Ready(None) => HealthState::Failed,
    };

    let _previous = self.sender.send_replace(next);
  }

  pub(crate) fn fail(&self) {
    self.transition_active_to(HealthState::Failed);
  }

  fn stop(&self) {
    self.transition_active_to(HealthState::Stopped);
  }

  fn transition_active_to(&self, next: HealthState) {
    if matches!(
      *self.sender.borrow(),
      HealthState::Starting | HealthState::Running
    ) {
      let _previous = self.sender.send_replace(next);
    }
  }
}

impl Drop for HealthLifecycle {
  fn drop(&mut self) {
    self.stop();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn starts_in_starting_state() {
    let lifecycle = HealthLifecycle::new();

    assert_eq!(lifecycle.health_check().state(), HealthState::Starting);
  }

  #[test]
  fn pending_first_poll_marks_consumer_running() {
    let lifecycle = HealthLifecycle::new();
    let poll: Poll<Option<Result<(), ()>>> = Poll::Pending;

    lifecycle.observe_first_poll(&poll);

    assert_eq!(lifecycle.health_check().state(), HealthState::Running);
  }

  #[test]
  fn cloned_health_checks_observe_the_same_transition() {
    let lifecycle = HealthLifecycle::new();
    let first = lifecycle.health_check();
    let second = first.clone();

    lifecycle.observe_first_poll(&Poll::<Option<Result<(), ()>>>::Pending);

    assert_eq!(first.state(), HealthState::Running);
    assert_eq!(second.state(), HealthState::Running);
  }

  #[test]
  fn successful_first_poll_marks_consumer_running() {
    let lifecycle = HealthLifecycle::new();
    let poll: Poll<Option<Result<(), ()>>> = Poll::Ready(Some(Ok(())));

    lifecycle.observe_first_poll(&poll);

    assert_eq!(lifecycle.health_check().state(), HealthState::Running);
  }

  #[test]
  fn failed_first_poll_never_marks_consumer_running() {
    let lifecycle = HealthLifecycle::new();
    let poll: Poll<Option<Result<(), ()>>> = Poll::Ready(Some(Err(())));

    lifecycle.observe_first_poll(&poll);

    assert_eq!(lifecycle.health_check().state(), HealthState::Failed);
  }

  #[test]
  fn closed_first_poll_never_marks_consumer_running() {
    let lifecycle = HealthLifecycle::new();
    let poll: Poll<Option<Result<(), ()>>> = Poll::Ready(None);

    lifecycle.observe_first_poll(&poll);

    assert_eq!(lifecycle.health_check().state(), HealthState::Failed);
  }

  #[test]
  fn dropping_starting_consumer_marks_it_stopped() {
    let lifecycle = HealthLifecycle::new();
    let health = lifecycle.health_check();

    drop(lifecycle);

    assert_eq!(health.state(), HealthState::Stopped);
  }

  #[test]
  fn dropping_running_consumer_marks_it_stopped() {
    let lifecycle = HealthLifecycle::new();
    let health = lifecycle.health_check();

    lifecycle.observe_first_poll(&Poll::<Option<Result<(), ()>>>::Pending);
    drop(lifecycle);

    assert_eq!(health.state(), HealthState::Stopped);
  }

  #[test]
  fn failed_state_is_terminal() {
    let lifecycle = HealthLifecycle::new();
    let health = lifecycle.health_check();

    lifecycle.fail();
    lifecycle.observe_first_poll(&Poll::<Option<Result<(), ()>>>::Pending);
    drop(lifecycle);

    assert_eq!(health.state(), HealthState::Failed);
  }

  #[test]
  fn stopped_state_is_terminal() {
    let lifecycle = HealthLifecycle::new();
    let health = lifecycle.health_check();

    lifecycle.stop();
    lifecycle.observe_first_poll(&Poll::<Option<Result<(), ()>>>::Pending);

    assert_eq!(health.state(), HealthState::Stopped);
  }
}
