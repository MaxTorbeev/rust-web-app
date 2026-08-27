use uuid::Uuid;

/// Уникальный ключ обработки события внутри одной логической области consumer-а.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DedupKey {
  scope: String,
  event_id: Uuid,
}

/// Подтверждает исключительное право конкретного исполнителя на обработку ключа.
#[derive(Debug, Eq, PartialEq)]
pub struct DedupLease {
  key: DedupKey,
  token: Uuid,
}

impl DedupKey {
  pub fn new(scope: impl Into<String>, event_id: Uuid) -> Self {
    Self {
      scope: scope.into(),
      event_id,
    }
  }

  pub fn scope(&self) -> &str {
    &self.scope
  }

  pub const fn event_id(&self) -> Uuid {
    self.event_id
  }
}

impl DedupLease {
  pub fn new(key: DedupKey, token: Uuid) -> Self {
    Self { key, token }
  }

  pub fn key(&self) -> &DedupKey {
    &self.key
  }

  pub const fn token(&self) -> Uuid {
    self.token
  }
}
