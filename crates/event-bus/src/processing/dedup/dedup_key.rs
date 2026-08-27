use uuid::Uuid;

pub struct DedupKey {
  scope: String,
  event_id: Uuid,
}

pub struct DedupLease {
  key: DedupKey,
  token: Uuid
}

impl DedupKey {
  pub fn new(scope: String, event_id: Uuid) -> Self {
    Self { scope, event_id }
  }
}