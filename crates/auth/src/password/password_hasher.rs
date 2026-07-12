use argon2::{Argon2, PasswordHasher as ArgonPasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, Error, SaltString, PasswordHash};

pub struct PasswordHasher;

impl PasswordHasher {
    pub fn make(password: &str) -> Result<String, Error> {
        let salt = SaltString::generate(&mut OsRng);

        let argon2 = Argon2::default();

        let hash = argon2.hash_password(password.as_bytes(), &salt)?;

        Ok(hash.to_string())
    }

    pub fn verify(password: &str, hash: &str) -> bool {
        let parsed_hash = match PasswordHash::new(hash) {
            Ok(parsed_hash) => parsed_hash,
            Err(_) => return false,
        };

        Argon2::default()
          .verify_password(password.as_bytes(), &parsed_hash)
          .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::PasswordHasher;

    #[test]
    fn make_returns_hash_that_can_be_verified() {
        let password = "secert";

        let hash = PasswordHasher::make(password).unwrap();

        println!("password hash {}", hash);

        assert!(PasswordHasher::verify(password, &hash));
    }
}
