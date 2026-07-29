use serde_json::Error;

#[derive(Debug)]
pub enum TokenIssueError {
  EmptyClientId,
  CapabilitySerialization(serde_json::Error),
  TokenEncoding(jsonwebtoken::errors::Error),
}

impl From<Error> for TokenIssueError {
  fn from(error: Error) -> Self {
    Self::CapabilitySerialization(error)
  }
}