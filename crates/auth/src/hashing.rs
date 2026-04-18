use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use ennoia_kernel::AuthError;
use rand::{rngs::OsRng, TryRngCore};

pub fn hash_password(plain: &str) -> Result<String, AuthError> {
    let mut salt_bytes = [0u8; 16];
    OsRng
        .try_fill_bytes(&mut salt_bytes)
        .map_err(|e| AuthError::Backend(e.to_string()))?;
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|e| AuthError::Backend(e.to_string()))?;
    let argon = Argon2::default();
    let hashed = argon
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| AuthError::Backend(e.to_string()))?;
    Ok(hashed.to_string())
}

pub fn verify_password(plain: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed = PasswordHash::new(hash).map_err(|e| AuthError::Invalid(e.to_string()))?;
    match Argon2::default().verify_password(plain.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(AuthError::Backend(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify() {
        let hashed = hash_password("s3cret!").unwrap();
        assert!(verify_password("s3cret!", &hashed).unwrap());
        assert!(!verify_password("wrong", &hashed).unwrap());
    }
}
