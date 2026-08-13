#[derive(Debug, Clone)]
pub struct NatsConfig {
  pub servers: Vec<String>,
}

impl NatsConfig {
  pub fn new(servers: Vec<String>) -> Self {
    Self {
      servers
    }
  }
}