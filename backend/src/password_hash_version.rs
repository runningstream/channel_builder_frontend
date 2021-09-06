use std::fmt;

const CUR_HASH_VER: i32 = 1;

#[derive(Debug, Clone)]
pub enum CustError { InvalidHashVer, HashingError, HashParseError }

impl fmt::Display for CustError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CustError::InvalidHashVer => write!(f, "Invalid hash version supplied"),
            CustError::HashingError => write!(f, "Error when hashing..."),
            CustError::HashParseError => write!(f, "Error parsing hash..."),
        }
    }
}

// Hash a password as the current best version
pub fn hash_pw(username: &str, password: &str) -> Result<String, CustError> {
    hash_pw_ver(username, password, CUR_HASH_VER)
}

// Get the current best hash version
pub fn get_pw_ver() -> i32 { CUR_HASH_VER }

// Hash a password as a specific version
pub fn hash_pw_ver(_username: &str, password: &str, version: i32)
    -> Result<String, CustError>
{
    match version {
        1 => hash_pw_ver_1(password),
        _ => Err(CustError::InvalidHashVer),
    }
}

// Validate a password against a specific version
pub fn validate_pw_ver(_username: &str, password: &str,
    password_hash: &str, version: i32) -> Result<bool, CustError>
{
    match version {
        1 => validate_pw_ver_1(password, password_hash),
        _ => Err(CustError::InvalidHashVer),
    }
}

fn hash_pw_ver_1(password: &str) -> Result<String, CustError> {
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::Argon2;

    // Get the salt...
    let salt = SaltString::generate(&mut rand::thread_rng());

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Hash password to PHC string ($argon2id$v=19$...)
    let hash_result = argon2.hash_password_simple(password.as_bytes(),
        salt.as_ref());
    match hash_result {
        Ok(hash) => Ok(hash.to_string()),
        Err(err) => {
            println!("Error in hashing: {}", err);
            Err(CustError::HashingError)
            },
    }
}

fn validate_pw_ver_1(password: &str, password_hash: &str)
    -> Result<bool, CustError>
{
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    use argon2::Argon2;

    // Parse the hash to grab the salt and properties
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(val) => val,
        Err(err) => {
            println!("Error in parsing hash: {}", err);
            return Err(CustError::HashParseError);
            },
    };

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Verify password
    Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
}
