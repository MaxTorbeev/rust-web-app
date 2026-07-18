use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::Sha256;
use sha2::Digest;

pub struct Token {

}

impl Token {
  pub fn generate() -> Result<String, getrandom::Error> {
    let mut buf = [0u8; 32];

    getrandom::fill(&mut buf)?;

    Ok(URL_SAFE_NO_PAD.encode(buf))
  }

  pub fn fingerprint(token: &str) -> String {
    let hash = Sha256::digest(token.as_bytes());

    hex::encode(hash)
  }
}