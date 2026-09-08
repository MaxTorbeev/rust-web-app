//! Ключ lease, его владелец и token периода владения — публичные типы
//! аргументов операций.
//!
//! Ключ и владелец — непрозрачные непустые строки: примитив не знает, что за
//! ними стоит. Разные типы намеренно: аргументы операций строковые, и
//! `acquire(owner, key, ttl)` с перепутанными аргументами иначе компилировался
//! бы молча.

use std::fmt;

use crate::error::RedisLeaseError;
use crate::outcome::Fence;

/// Общее представление [`LeaseKey`] и [`LeaseOwner`]: непрозрачная непустая
/// строка.
///
/// Пустое значение Redis принял бы молча, но все вызывающие с пустым ключом
/// делили бы один lease, а с пустым владельцем были бы неотличимы друг от
/// друга. Поэтому проверка одна и живёт здесь, а не в каждой роли.
#[derive(Clone, Eq, PartialEq)]
struct NonEmpty(String);

impl NonEmpty {
  fn new(value: impl Into<String>, field: &'static str) -> Result<Self, RedisLeaseError> {
    let value = value.into();

    if value.is_empty() {
      return Err(RedisLeaseError::Empty { field });
    }

    Ok(Self(value))
  }

  fn as_str(&self) -> &str {
    &self.0
  }
}

/// Представление не должно быть видно снаружи: `LeaseKey("k")`, а не
/// `LeaseKey(NonEmpty("k"))`.
impl fmt::Debug for NonEmpty {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(&self.0, f)
  }
}

/// Ключ lease. Непрозрачен для примитива; пространство имён — забота вызывающего.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaseKey(NonEmpty);

impl LeaseKey {
  pub fn new(key: impl Into<String>) -> Result<Self, RedisLeaseError> {
    NonEmpty::new(key, "key").map(Self)
  }

  pub fn as_str(&self) -> &str {
    self.0.as_str()
  }
}

/// Владелец lease. Непрозрачная непустая строка: `node_id:boot_generation`,
/// идентификатор процесса publisher-а, токен запуска reaper-а.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaseOwner(NonEmpty);

impl LeaseOwner {
  pub fn new(owner: impl Into<String>) -> Result<Self, RedisLeaseError> {
    NonEmpty::new(owner, "owner").map(Self)
  }

  pub fn as_str(&self) -> &str {
    self.0.as_str()
  }
}

/// Идентичность одного периода владения: какой ключ, кто держит и fence,
/// выданный Redis при захвате.
///
/// Владелец один и тот же во всех своих периодах, поэтому владельца недостаточно:
/// если бы в Redis лежало только `lease:<owner>`, отложенный повтор `release`
/// из прошлого периода удалил бы новый lease того же владельца, а работа,
/// начатая в прошлом периоде, прошла бы `holds_lease` в новом. Fence разных
/// периодов различаются, поэтому `renew`, `release` и `holds_lease` сверяют
/// token целиком, и token прошлого периода не совпадает с текущим значением.
///
/// Token непрерывного периода стабилен: повторный `acquire` тем же владельцем,
/// пока lease не истёк и не освобождён, возвращает тот же fence. Потерянный
/// ответ Redis не превращает retry в новый период.
///
/// Token выдаёт только [`crate::RedisLease::acquire`]: собрать его из ключа,
/// владельца и числа снаружи нельзя, как и [`Fence`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaseToken {
  key: LeaseKey,
  owner: LeaseOwner,
  fence: Fence,
}

impl LeaseToken {
  pub(crate) fn new(key: LeaseKey, owner: LeaseOwner, fence: Fence) -> Self {
    Self { key, owner, fence }
  }

  pub fn key(&self) -> &LeaseKey {
    &self.key
  }

  pub fn owner(&self) -> &LeaseOwner {
    &self.owner
  }

  pub fn fence(&self) -> Fence {
    self.fence
  }
}
