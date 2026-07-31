use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebsocketFormat {
  Json,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WebSocketQuery {
  pub access_token: String,
  pub format: WebsocketFormat,
  pub heartbeats: bool
}