#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use nom_app::app::*;
    use nom_app::auth::middleware::{AuthConfig, auth_middleware};
    use nom_app::log::middleware::log_middleware;
    use std::sync::Arc;
    use tower_http::compression::CompressionLayer;

    let auth_config = match AuthConfig::from_env() {
        Ok(config) => Arc::new(config),
        Err(message) => {
            eprintln!("nom: {message}");
            std::process::exit(1);
        }
    };

    if let AuthConfig::Disabled = *auth_config {
        log!("Authentication is disabled (NOM_NO_AUTH)");
    }

    if let Err(error) = nom_app::db::init() {
        eprintln!("nom: can't open the database: {error}");
        std::process::exit(1);
    }

    let conf = get_configuration(Some("./Cargo.toml")).unwrap();

    log!("Using config: {:#?}", conf.leptos_options);

    let addr = conf.leptos_options.site_addr;
    let mut leptos_options = conf.leptos_options;

    // The Docker build sets LEPTOS_HASH_FILES=true, so cargo-leptos puts content hashes in the
    // JS/WASM/CSS file names. Read when compiling, so the server matches its build even where
    // the variable isn't set at runtime.
    if option_env!("LEPTOS_HASH_FILES").is_some_and(|v| v.eq_ignore_ascii_case("true")) {
        leptos_options.hash_files = true;
    }

    if let Err(message) = load_stylesheet(&leptos_options) {
        eprintln!("nom: {message}");
        std::process::exit(1);
    }

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    // Routes must be added before the layers, or the layers (auth!) don't apply to them
    let mut app = Router::new()
        .route(
            "/random",
            axum::routing::get(nom_app::recipe::random_recipe_redirect),
        )
        .route(
            "/images/{id}",
            axum::routing::get(nom_app::images::serve_image),
        );

    if leptos_options.hash_files {
        use axum::http::{HeaderValue, header::CACHE_CONTROL};
        use tower_http::set_header::SetResponseHeader;

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

    let app = app
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn_with_state(
            auth_config,
            auth_middleware,
        ))
        .layer(axum::middleware::from_fn(log_middleware))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
