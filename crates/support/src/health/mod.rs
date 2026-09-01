use std::future::Future;

/// Result of a component health verification.
pub trait HealthReport {
  /// Returns whether the component satisfies its health requirements.
  fn is_healthy(&self) -> bool;
}

/// Verifies a component's current health without changing its state.
///
/// # Examples
///
/// ```
/// use support::health::{HealthReport, VerifyHealth};
///
/// async fn is_healthy(check: &impl VerifyHealth) -> bool {
///   check.verify().await.is_healthy()
/// }
/// ```
pub trait VerifyHealth: Send + Sync {
  type Report: HealthReport + Send;

  fn verify(&self) -> impl Future<Output = Self::Report> + Send + '_;
}
