use tower::Layer;

use super::{service::AuthService, AuthState};

#[derive(Clone)]
pub struct AuthLayer {
    state: AuthState,
}

impl AuthLayer {
    pub fn new(state: AuthState) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            state: self.state.clone(),
        }
    }
}
