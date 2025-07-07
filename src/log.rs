#[cfg(feature = "ssr")]
pub mod middleware {
    use axum::extract::Request;
    use axum::http::StatusCode;
    use axum::middleware::Next;
    use axum::response::Response;
    use leptos::logging::log;

    pub async fn log_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
        let req_log_str = format!("{} {}", request.method(), request.uri());

        let response = next.run(request).await;

        let resp_log_str = format!("{}", response.status());

        log!("{} -> {}", req_log_str, resp_log_str);

        Ok(response)
    }
}
