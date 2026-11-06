pub mod app;
#[cfg(feature = "ssr")]
pub mod auth;
#[cfg(feature = "ssr")]
pub mod db;
pub mod images;
#[cfg(feature = "ssr")]
pub mod markdown;
pub mod pages;
pub mod recipe;
#[cfg(feature = "ssr")]
pub mod search;
#[cfg(feature = "ssr")]
pub mod shell;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
