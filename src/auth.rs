#[cfg(feature = "ssr")]
pub mod middleware {
    use std::env::VarError;
    use std::sync::Arc;

    use axum::body::Body;
    use axum::extract::{Request, State};
    use axum::http::StatusCode;
    use axum::middleware::Next;
    use axum::response::Response;
    use axum_extra::TypedHeader;
    use axum_extra::headers::Authorization;
    use axum_extra::headers::authorization::Basic;

    /// How requests are authenticated. Read once at startup by [`AuthConfig::from_env`].
    pub enum AuthConfig {
        /// `NOM_NO_AUTH` is set: every request is let through.
        Disabled,
        /// HTTP Basic auth with the credentials from `NOM_USERNAME` and `NOM_PASSWORD`.
        Basic { username: String, password: String },
    }

    impl AuthConfig {
        /// Reads `NOM_NO_AUTH`, `NOM_USERNAME` and `NOM_PASSWORD`.
        ///
        /// Exactly one of these must hold, otherwise this returns an error to show the user:
        /// - `NOM_NO_AUTH` is `1` or `true` (any case), and both credentials are unset.
        /// - `NOM_NO_AUTH` is unset, and both credentials are set.
        ///
        /// An empty variable counts as unset.
        pub fn from_env() -> Result<Self, String> {
            let no_auth = match env("NOM_NO_AUTH")? {
                None => false,
                Some(v) if v == "1" || v.eq_ignore_ascii_case("true") => true,
                Some(v) => return Err(format!("NOM_NO_AUTH must be `1` or `true`, got `{v}`")),
            };

            match (no_auth, env("NOM_USERNAME")?, env("NOM_PASSWORD")?) {
                (true, None, None) => Ok(Self::Disabled),
                (true, _, _) => Err(
                    "NOM_NO_AUTH can't be combined with NOM_USERNAME or NOM_PASSWORD; unset one of them"
                        .to_string(),
                ),
                (false, Some(username), Some(password)) => Ok(Self::Basic { username, password }),
                (false, _, _) => Err(
                    "set both NOM_USERNAME and NOM_PASSWORD, or set NOM_NO_AUTH=true to run without authentication"
                        .to_string(),
                ),
            }
        }
    }

    /// The variable's value, or `None` if it's unset or empty.
    fn env(name: &str) -> Result<Option<String>, String> {
        match std::env::var(name) {
            Ok(v) if v.is_empty() => Ok(None),
            Ok(v) => Ok(Some(v)),
            Err(VarError::NotPresent) => Ok(None),
            Err(VarError::NotUnicode(_)) => Err(format!("{name} is not valid UTF-8")),
        }
    }

    /// Takes the same time for every pair of equal-length inputs, so the response time doesn't
    /// reveal how many leading bytes of a guess were right.
    fn constant_time_eq(a: &str, b: &str) -> bool {
        a.len() == b.len()
            && a.bytes()
                .zip(b.bytes())
                .fold(0, |acc, (x, y)| acc | (x ^ y))
                == 0
    }

    fn auth_required() -> Response {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("WWW-Authenticate", "Basic realm=\"nom\"")
            .body(Body::empty())
            .unwrap()
    }

    pub async fn auth_middleware(
        State(config): State<Arc<AuthConfig>>,
        auth_header: Option<TypedHeader<Authorization<Basic>>>,
        request: Request,
        next: Next,
    ) -> Response {
        let AuthConfig::Basic { username, password } = &*config else {
            return next.run(request).await;
        };

        let Some(TypedHeader(auth)) = auth_header else {
            return auth_required();
        };

        // `&` instead of `&&`, so the password is compared even when the username is wrong.
        if constant_time_eq(auth.username(), username) & constant_time_eq(auth.password(), password)
        {
            next.run(request).await
        } else {
            leptos::logging::log!("Login failed for user {:?}", auth.username());
            auth_required()
        }
    }
}
