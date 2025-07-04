#[cfg(feature = "ssr")]
pub mod middleware {
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::StatusCode;
    use axum::middleware::Next;
    use axum::response::Response;
    use axum_extra::TypedHeader;
    use axum_extra::headers::Authorization;
    use axum_extra::headers::authorization::Basic;

    fn auth_required() -> Response {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("WWW-Authenticate", "Basic realm=\"nom\"")
            .body(Body::empty())
            .unwrap()
    }

    pub async fn auth_middleware(
        auth_header: Option<TypedHeader<Authorization<Basic>>>,
        request: Request,
        next: Next,
    ) -> Result<Response, StatusCode> {
        macro_rules! auth_ok {
            () => {
                next.run(request).await
            };
        }

        let username_env = std::env::var("NOM_USERNAME").ok();
        let password_env = std::env::var("NOM_PASSWORD").ok();

        let (username, password) = if let Some((u, p)) = username_env.zip(password_env) {
            (u, p)
        } else {
            return Ok(auth_ok!());
        };

        if let Some(auth) = auth_header {
            if auth.username() != username && auth.password() != password {
                return Ok(auth_required());
            }
        } else {
            return Ok(auth_required());
        }

        Ok(auth_ok!())
    }
}
