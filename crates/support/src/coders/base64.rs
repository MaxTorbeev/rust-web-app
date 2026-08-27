use crate::Base64Error;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

pub fn decode(value: &str) -> Result<Vec<u8>, Base64Error> {
  STANDARD
    .decode(value)
    .map_err(|_| Base64Error::InvalidInput)
}
pub fn decode_to_string(value: &str) -> Result<String, Base64Error> {
  let decoded = decode(value)?;

  String::from_utf8(decoded).map_err(|_| Base64Error::InvalidUtf8)
}
pub fn encode(value: impl AsRef<[u8]>) -> String {
  STANDARD.encode(value)
}
