use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use crate::{ApiMessage, ApiResponse};

pub struct ApiError {
  status: StatusCode,
  message: String,
}

impl ApiError {
  pub fn internal(message: &str) -> Self {
    Self {
      status: StatusCode::INTERNAL_SERVER_ERROR,
      message: message.to_owned(),
    }
  }

  pub fn unauthorized(message: &str) -> Self {
    Self {
      status: StatusCode::UNAUTHORIZED,
      message: message.to_owned(),
    }
  }
}

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    (
      self.status,
      Json(ApiResponse::new(ApiMessage::new(self.message))),
    ).into_response()
  }
}
