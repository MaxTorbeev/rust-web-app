use base64::DecodeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base64Error {
  InvalidInput,
  InvalidUtf8,
}

impl From<DecodeError> for Base64Error {
  fn from(_error: DecodeError) -> Self {
    Self::InvalidInput
  }
}

impl std::fmt::Display for Base64Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Invalid base64 input")
  }
}

impl std::error::Error for Base64Error {}
