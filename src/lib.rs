pub mod app;
pub mod auth;
#[cfg(feature = "ssr")]
pub mod db;
pub mod log;
#[cfg(feature = "ssr")]
pub mod markdown;
pub mod pages;
pub mod recipe;
#[cfg(feature = "ssr")]
pub mod search;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
