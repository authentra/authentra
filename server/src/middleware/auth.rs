mod future;
mod layer;
mod service;

use std::sync::Arc;

use jsonwebtoken::{DecodingKey, EncodingKey};
pub use layer::AuthLayer;
use rand::{rngs::adapter::ReseedingRng, SeedableRng};
use rand_chacha::{rand_core::OsRng, ChaCha20Core};
use tokio::sync::Mutex;

pub type AuthRng = ReseedingRng<ChaCha20Core, OsRng>;

#[derive(Clone)]
pub struct AuthState {
    rng: Arc<Mutex<AuthRng>>,
    encoding: Arc<EncodingKey>,
    decoding: Arc<DecodingKey>,
}

impl AuthState {
    pub fn new(secret: String) -> Self {
        let chacha = ChaCha20Core::from_entropy();
        let rng: AuthRng = ReseedingRng::new(chacha, 512, OsRng);
        Self {
            rng: Arc::new(Mutex::new(rng)),
            encoding: Arc::new(EncodingKey::from_secret(secret.as_bytes())),
            decoding: Arc::new(DecodingKey::from_secret(secret.as_bytes())),
        }
    }

    pub fn encoding(&self) -> &EncodingKey {
        &self.encoding
    }
    pub fn decoding(&self) -> &DecodingKey {
        &self.decoding
    }

    pub async fn with_rng<T, F: FnOnce(&mut AuthRng) -> T>(&self, func: F) -> T {
        let mut rng = self.rng.lock().await;
        func(&mut *rng)
    }
}
