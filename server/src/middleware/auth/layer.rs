use tower::Layer;

use super::{service::AuthService, AuthState};

pub struct AuthLayer {
    state: AuthState,
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
