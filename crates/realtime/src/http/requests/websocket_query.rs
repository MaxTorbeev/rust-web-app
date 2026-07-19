use serde::Deserialize;

#[derive(Deserialize)]
pub struct WebSocketQuery {
  pub token: String
}