use thiserror::Error;

const CUR_HASH_VER: i32 = 1;

#[derive(Error, Debug, Clone)]
pub enum PWHashError {
    #[error("invalid hash version supplied")]
    InvalidHashVer,
    #[error("error when hashing: {err:?}")]
    HashingError { err: argon2::password_hash::HasherError },
    #[error("error parsing hash: {err:?}")]
    HashParseError { err: argon2::password_hash::HashError },
}

// Hash a password as the current best version
pub fn hash_pw(username: &str, password: &str) -> Result<String, PWHashError> {
    hash_pw_ver(username, password, CUR_HASH_VER)
}

// Get the current best hash version
pub fn get_pw_ver() -> i32 { CUR_HASH_VER }

// Hash a password as a specific version
pub fn hash_pw_ver(_username: &str, password: &str, version: i32)
    -> Result<String, PWHashError>
{
    match version {
        1 => hash_pw_ver_1(password),
        _ => Err(PWHashError::InvalidHashVer),
    }
}

// Validate a password against a specific version
pub fn validate_pw_ver(_username: &str, password: &str,
    password_hash: &str, version: i32) -> Result<bool, PWHashError>
{
    match version {
        1 => validate_pw_ver_1(password, password_hash),
        _ => Err(PWHashError::InvalidHashVer),
    }
}

fn hash_pw_ver_1(password: &str) -> Result<String, PWHashError> {
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::Argon2;

    // Get the salt...
    let salt = SaltString::generate(&mut rand::thread_rng());

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Hash password to PHC string ($argon2id$v=19$...)
    match argon2.hash_password_simple(
        password.as_bytes(),
        salt.as_ref()
    ) {
        Ok(hash) => Ok(hash.to_string()),
        Err(err) => Err(PWHashError::HashingError { err }),
    }
}

fn validate_pw_ver_1(password: &str, password_hash: &str)
    -> Result<bool, PWHashError>
{
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    use argon2::Argon2;

    // Parse the hash to grab the salt and properties
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|err| { PWHashError::HashParseError { err } })?;

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Verify password
    Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
}
