#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BroadcastOutcome {
  pub enqueued: usize,
  pub disconnected: usize,
}
