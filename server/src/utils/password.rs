use argon2::password_hash::Error as ArgonError;
use argon2::{
    password_hash::{Encoding, SaltString},
    Argon2, PasswordHash, PasswordHasher,
};
use once_cell::sync::Lazy;
use rand::thread_rng;

static ARGON2_INSTANCE: Lazy<Argon2> = Lazy::new(|| Argon2::default());

pub fn hash_password(password: &[u8]) -> Result<String, ArgonError> {
    let salt = SaltString::generate(thread_rng());
    ARGON2_INSTANCE
        .hash_password(password, &salt)
        .map(|hash| hash.to_string())
}

pub fn verify_password(hash: &str, password: &[u8]) -> Result<(), ArgonError> {
    let hash = PasswordHash::parse(hash, Encoding::B64)?;
    hash.verify_password(&[&*ARGON2_INSTANCE], password)
}

pub fn handle_result<T>(result: Result<T, ArgonError>) -> Result<Option<T>, ArgonError> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(err) => match err {
            ArgonError::Password => Ok(None),
            _ => Err(err),
        },
    }
}
