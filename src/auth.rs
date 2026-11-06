//! HTTP Basic auth for every request, and a guard against other sites submitting forms. Server
//! only.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// How requests are authenticated. Read once at startup by [`AuthConfig::from_env`].
pub enum AuthConfig {
    /// `NOM_NO_AUTH` is set: every request is let through.
    Disabled,
    /// HTTP Basic auth: the base64 of `<NOM_USERNAME>:<NOM_PASSWORD>`, as browsers send it.
    Basic(String),
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
        use base64::Engine;

        let no_auth = match env("NOM_NO_AUTH") {
            None => false,
            Some(v) if v == "1" || v.eq_ignore_ascii_case("true") => true,
            Some(v) => return Err(format!("NOM_NO_AUTH must be `1` or `true`, got `{v}`")),
        };

        match (no_auth, env("NOM_USERNAME"), env("NOM_PASSWORD")) {
            (true, None, None) => Ok(Self::Disabled),
            (true, _, _) => Err(
                "NOM_NO_AUTH can't be combined with NOM_USERNAME or NOM_PASSWORD; unset one of them"
                    .to_string(),
            ),
            (false, Some(username), Some(password)) => Ok(Self::Basic(
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}")),
            )),
            (false, _, _) => Err(
                "set both NOM_USERNAME and NOM_PASSWORD, or set NOM_NO_AUTH=true to run without authentication"
                    .to_string(),
            ),
        }
    }
}

/// The variable's value, or `None` if it's unset, empty or not UTF-8.
fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// Whether `header` is `Basic <expected>`. The scheme is case-insensitive.
fn authorized(header: Option<&HeaderValue>, expected: &str) -> bool {
    header
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.split_once(' '))
        .is_some_and(|(scheme, credentials)| {
            scheme.eq_ignore_ascii_case("basic") & constant_time_eq(credentials.trim(), expected)
        })
}

/// Takes the same time for every guess, so the response time reveals neither how many bytes
/// of a guess were right nor how long the secret is.
fn constant_time_eq(guess: &str, secret: &str) -> bool {
    let guess = guess.as_bytes();
    let diff = secret
        .bytes()
        .enumerate()
        .fold(guess.len() ^ secret.len(), |diff, (i, byte)| {
            diff | usize::from(guess.get(i).copied().unwrap_or(0) ^ byte)
        });

    diff == 0
}

/// Lets a request through only with valid credentials (unless auth is disabled), and only if
/// it's not a cross-site request that changes something.
pub async fn auth_middleware(
    State(config): State<Arc<AuthConfig>>,
    request: Request,
    next: Next,
) -> Response {
    if let AuthConfig::Basic(expected) = &*config
        && !authorized(request.headers().get(AUTHORIZATION), expected)
    {
        if request.headers().contains_key(AUTHORIZATION) {
            leptos::logging::log!("Login failed");
        }

        return (
            StatusCode::UNAUTHORIZED,
            [(WWW_AUTHENTICATE, "Basic realm=\"nom\"")],
        )
            .into_response();
    }

    // Browsers send saved Basic credentials along with a form that another site submits to
    // nom, so refuse those. Browsers mark the request; `none` means the user typed the URL.
    let cross_site = request
        .headers()
        .get("sec-fetch-site")
        .is_some_and(|site| site != "same-origin" && site != "none");

    if cross_site && !matches!(*request.method(), Method::GET | Method::HEAD) {
        return StatusCode::FORBIDDEN.into_response();
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_the_header() {
        let expected = "Y2hlZjpnZWhlaW0="; // chef:geheim
        let check =
            |header: &str| authorized(Some(&HeaderValue::from_str(header).unwrap()), expected);

        assert!(check("Basic Y2hlZjpnZWhlaW0="));
        assert!(check("basic Y2hlZjpnZWhlaW0="));
        assert!(!check("Basic Y2hlZjpnZWhlaW1="));
        assert!(!check("Basic Y2hlZjpnZWhlaW0"));
        assert!(!check("Basic Y2hlZjpnZWhlaW0=x"));
        assert!(!check("Bearer Y2hlZjpnZWhlaW0="));
        assert!(!check("Basic"));
        assert!(!check("Basic "));
        assert!(!authorized(None, expected));
    }
}
