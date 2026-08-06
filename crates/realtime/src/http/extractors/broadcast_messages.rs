use axum::extract::{FromRequest, Request};
use axum::extract::rejection::JsonRejection;
use axum::http::{StatusCode};
use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationErrors};
use api_response::{ApiMessage, ApiResponse};
use crate::requests::BroadcastMessage;

/// Декодированные сообщения одного HTTP publish-запроса.
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct BroadcastMessages {
  #[validate(
    length(
      min = 1,
      code = "empty_batch",
      message = "message batch must not be empty"
    ),
    nested
  )]
  messages: Vec<BroadcastMessage>,
}

impl BroadcastMessages {
  pub fn into_inner(self) -> Vec<BroadcastMessage> {
    self.messages
  }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OneOrMany<T> {
  One(T),
  Many(Vec<T>),
}

pub enum BroadcastMessagesRejection {
  Json(JsonRejection),
  Validation(ValidationErrors)
}

impl From<JsonRejection> for BroadcastMessagesRejection {
  fn from(error: JsonRejection) -> Self {
    Self::Json(error)
  }
}

impl From<ValidationErrors> for BroadcastMessagesRejection {
  fn from(error: ValidationErrors) -> Self {
    Self::Validation(error)
  }
}

impl IntoResponse for BroadcastMessagesRejection {
  fn into_response(self) -> Response {
    match self {
      Self::Json(error) => {
        error.into_response()
      }

      Self::Validation(errors) => {
        tracing::debug!(
          %errors,
          "broadcast messages validation failed"
        );

        (
          StatusCode::BAD_REQUEST,
          Json(ApiResponse::new(ApiMessage::new(
            "message batch must not be empty".to_owned(),
          ))),
        ).into_response()
      }
    }
  }
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
  type Rejection = BroadcastMessagesRejection;

  async fn from_request(
    req: Request,
    state: &S
  ) -> Result<Self, Self::Rejection> {
    let Json(payload) =
      Json::<OneOrMany<BroadcastMessage>>::from_request(
        req,
        state,
      ).await?;

    let messages = Self {
      messages: payload.into_vec()
    };

    messages.validate()?;

    Ok(messages)
  }
}