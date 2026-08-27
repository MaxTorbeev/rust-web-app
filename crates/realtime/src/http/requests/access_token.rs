use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenRequest {
  pub client_id: String,
}
