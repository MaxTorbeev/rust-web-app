use axum::extract::{FromRequest, Request};
use axum::extract::rejection::JsonRejection;
use axum::Json;
use serde::Deserialize;
use crate::requests::BroadcastMessage;

/// Декодированные сообщения одного HTTP publish-запроса.
pub struct BroadcastMessages (
  pub Vec<BroadcastMessage>
);

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OneOrMany<T> {
  One(T),
  Many(Vec<T>),
}

impl <T> OneOrMany<T> {
  fn into_vec(self) -> Vec<T> {
    match self {
      Self::One(message) => vec![message],
      Self::Many(messages) => messages,
    }
  }
}

impl <S> FromRequest<S> for BroadcastMessages
where
  S: Send + Sync
{
  type Rejection = JsonRejection;

  async fn from_request(
    req: Request,
    state: &S
  ) -> Result<Self, Self::Rejection> {
    let Json(payload) =
      Json::<OneOrMany<BroadcastMessage>>::from_request(
        req,
        state,
      ).await?;

    Ok(Self(payload.into_vec()))
  }
}