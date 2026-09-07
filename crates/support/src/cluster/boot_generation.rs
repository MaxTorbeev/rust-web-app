use crate::fresh_uuid;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Идентификатор процесса внутри кластера.
/// Идентификатор изменяется после каждого запуска процесса приложения.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct BootGeneration(Uuid);

impl BootGeneration {
  pub fn generate() -> Self {
    Self(fresh_uuid())
  }

  /// Восстанавливает generation из известного UUID — для тестов и чтения
  /// сохранённых значений.
  pub const fn from_uuid(value: Uuid) -> Self {
    Self(value)
  }

  pub fn as_uuid(&self) -> &Uuid {
    &self.0
  }
}
