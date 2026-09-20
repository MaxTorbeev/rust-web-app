use super::lease::NodeLease;
use std::time::Duration;

/// Результат захвата lease конкретного запуска ноды.
#[derive(Debug)]
pub enum NodeClaimOutcome {
  /// Lease захвачен, поколение и deadline записаны в Redis.
  Acquired { lease: NodeLease },

  /// Lease этого node ID занят другим запуском.
  Held { remaining: Duration },
}
