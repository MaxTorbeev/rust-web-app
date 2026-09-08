use std::time::Duration;

use crate::identity::LeaseToken;

/// Монотонный номер периода владения.
///
/// Растёт при каждом новом захвате lease — в том числе тем же владельцем после
/// release или истечения — и никогда не уменьшается. Владелец, потерявший
/// lease, всегда держит меньший fence, чем тот, кто захватил lease после него,
/// поэтому внешние системы могут отвергать действия со старым fence.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Fence(u64);

impl Fence {
  pub const fn get(self) -> u64 {
    self.0
  }

  pub(crate) const fn new(value: u64) -> Self {
    Self(value)
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AcquireOutcome {
  /// Lease принадлежит вызывающему: захвачен сейчас либо уже был его.
  ///
  /// Повторный `acquire` тем же владельцем в непрерывном периоде продлевает
  /// TTL и возвращает тот же token, так что потерянный ответ Redis не
  /// превращается ни в потерю lease, ни в новый период. Token предъявляется в
  /// `renew`, `release` и `holds_lease`.
  Acquired { token: LeaseToken },

  /// Lease держит другой владелец; `remaining` — сколько ему осталось, если он
  /// не продлит.
  Held { remaining: Duration },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenewOutcome {
  Renewed,

  /// Lease истёк или принадлежит другому владельцу. Вызывающий обязан
  /// прекратить действия, защищённые этим lease.
  Lost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseOutcome {
  Released,

  /// Lease уже не принадлежал вызывающему; ничего не удалено.
  Lost,
}
