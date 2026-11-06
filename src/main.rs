#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use std::fmt::Display;
    use std::sync::Arc;

    use axum::Router;
    use axum::extract::Request;
    use axum::http::HeaderValue;
    use axum::http::header::{CACHE_CONTROL, REFERRER_POLICY, X_FRAME_OPTIONS};
    use axum::middleware::{Next, from_fn, from_fn_with_state};
    use axum::response::Response;
    use axum::routing::get;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use nom_app::app::App;
    use nom_app::auth::{AuthConfig, auth_middleware};
    use nom_app::shell::{load_stylesheet, shell};
    use tower_http::catch_panic::CatchPanicLayer;
    use tower_http::compression::CompressionLayer;
    use tower_http::set_header::{SetResponseHeader, SetResponseHeaderLayer};

    fn fail(message: impl Display) -> ! {
        eprintln!("nom: {message}");
        std::process::exit(1)
    }

    async fn log_request(request: Request, next: Next) -> Response {
        let line = format!("{} {}", request.method(), request.uri());
        let response = next.run(request).await;
        log!("{line} -> {}", response.status());
        response
    }

    let auth_config = Arc::new(AuthConfig::from_env().unwrap_or_else(|e| fail(e)));
    if let AuthConfig::Disabled = *auth_config {
        log!("Authentication is disabled (NOM_NO_AUTH)");
    }

    nom_app::db::init().unwrap_or_else(|e| fail(format!("can't open the database: {e}")));

    // From `LEPTOS_*` environment variables, with defaults for the rest. cargo-leptos builds
    // the output name into the binary, so it needs no Cargo.toml at runtime.
    let mut leptos_options = get_configuration(None)
        .unwrap_or_else(|e| fail(e))
        .leptos_options;

    // The Docker build sets LEPTOS_HASH_FILES=true, so cargo-leptos puts content hashes in the
    // JS/WASM/CSS file names. Read when compiling, so the server matches its build even where
    // the variable isn't set at runtime.
    if option_env!("LEPTOS_HASH_FILES").is_some_and(|v| v.eq_ignore_ascii_case("true")) {
        leptos_options.hash_files = true;
    }

    load_stylesheet(&leptos_options).unwrap_or_else(|e| fail(e));

    // Routes must be added before the layers, or the layers (auth!) don't apply to them
    let mut app = Router::new()
        .route("/random", get(nom_app::recipe::random_recipe_redirect))
        .route("/images/{id}", get(nom_app::images::serve_image));

    if leptos_options.hash_files {
        // A hashed name changes with the content, so browsers may keep these files forever.
        // `private`: everything is behind auth, so shared caches mustn't keep them.
        let pkg_files = SetResponseHeader::overriding(
            leptos_axum::site_pkg_dir_service(&leptos_options),
            CACHE_CONTROL,
            HeaderValue::from_static("private, max-age=31536000, immutable"),
        );

        app = app.route_service(
            &leptos_axum::site_pkg_dir_service_route_path(&leptos_options),
            pkg_files,
        );
    }

    let addr = leptos_options.site_addr;
    let app = app
        .leptos_routes(&leptos_options, generate_route_list(App), {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(CompressionLayer::new())
        // A panicking request gets a 500 instead of a dropped connection
        .layer(CatchPanicLayer::new())
        // No other site may show nom in a frame, or learn which nom page linked to it
        .layer(SetResponseHeaderLayer::overriding(
            X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            REFERRER_POLICY,
            HeaderValue::from_static("same-origin"),
        ))
        .layer(from_fn_with_state(auth_config, auth_middleware))
        .layer(from_fn(log_request))
        .with_state(leptos_options);

    log!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(not(feature = "ssr"))]
fn main() {}
