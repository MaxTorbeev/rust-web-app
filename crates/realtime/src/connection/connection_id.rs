pub struct ConnectionId(String);

impl ConnectionId {
  pub fn generate() -> Self {
    Self(uuid::Uuid::new_v4().to_string())
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }
}