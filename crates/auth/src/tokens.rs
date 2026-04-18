use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{Duration, Utc};
use ennoia_kernel::{AuthError, User};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::{rngs::OsRng, TryRngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// API keys are human-readable tokens beginning with `ek_`.
pub const API_KEY_PREFIX: &str = "ek_";

/// JWT claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,          // user id
    pub username: String,
    pub role: String,
    pub iat: i64,
    pub exp: i64,
}

/// Generate a 32-byte random session token (base64url, no padding).
pub fn generate_session_token() -> Result<String, AuthError> {
    let mut buf = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut buf)
        .map_err(|e| AuthError::Backend(e.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

/// Generate a 32-byte random API key prefixed with `ek_`.
pub fn generate_api_key() -> Result<String, AuthError> {
    let mut buf = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut buf)
        .map_err(|e| AuthError::Backend(e.to_string()))?;
    Ok(format!("{}{}", API_KEY_PREFIX, URL_SAFE_NO_PAD.encode(buf)))
}

/// Hash a token with SHA-256 (for session + api_key storage).
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

/// Mint a JWT for a user with the given TTL.
pub fn mint_jwt(user: &User, secret: &str, ttl_seconds: u32) -> Result<String, AuthError> {
    if secret.is_empty() {
        return Err(AuthError::Invalid("jwt secret not configured".to_string()));
    }
    let now = Utc::now();
    let claims = JwtClaims {
        sub: user.id.clone(),
        username: user.username.clone(),
        role: user.role.as_str().to_string(),
        iat: now.timestamp(),
        exp: (now + Duration::seconds(ttl_seconds as i64)).timestamp(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AuthError::Backend(e.to_string()))
}

/// Verify a JWT and return its claims.
pub fn verify_jwt(token: &str, secret: &str) -> Result<JwtClaims, AuthError> {
    if secret.is_empty() {
        return Err(AuthError::Invalid("jwt secret not configured".to_string()));
    }
    let data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::Expired,
        _ => AuthError::InvalidCredentials,
    })?;
    Ok(data.claims)
}

/// Generate a long random secret suitable for signing JWTs.
pub fn generate_jwt_secret() -> Result<String, AuthError> {
    let mut buf = [0u8; 64];
    OsRng
        .try_fill_bytes(&mut buf)
        .map_err(|e| AuthError::Backend(e.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ennoia_kernel::UserRole;

    fn sample_user() -> User {
        User {
            id: "user-1".to_string(),
            username: "admin".to_string(),
            display_name: None,
            email: None,
            role: UserRole::Admin,
            owner_kind: None,
            owner_id: None,
            created_at: "".to_string(),
            updated_at: "".to_string(),
            last_login_at: None,
        }
    }

    #[test]
    fn session_token_is_non_empty() {
        let t = generate_session_token().unwrap();
        assert!(t.len() >= 32);
    }

    #[test]
    fn api_key_has_prefix() {
        let k = generate_api_key().unwrap();
        assert!(k.starts_with(API_KEY_PREFIX));
    }

    #[test]
    fn hash_is_stable() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
    }

    #[test]
    fn jwt_round_trip() {
        let secret = "super-secret-for-testing";
        let token = mint_jwt(&sample_user(), secret, 60).unwrap();
        let claims = verify_jwt(&token, secret).unwrap();
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.role, "admin");
    }
}
