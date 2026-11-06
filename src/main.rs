#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use nom::app::*;
    use nom::auth::middleware::{AuthConfig, auth_middleware};
    use nom::log::middleware::log_middleware;
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

    let conf = get_configuration(Some("./Cargo.toml")).unwrap();

    log!("Using config: {:#?}", conf.leptos_options);

    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
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
