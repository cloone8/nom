pub mod app;
pub mod auth;
pub mod pages;
pub mod recipe;

#[cfg(feature = "ssr")]
pub static DB: std::sync::LazyLock<tokio::sync::Mutex<rusqlite::Connection>> =
    std::sync::LazyLock::new(|| {
        let db_path = std::env::var("NOM_DB").ok().unwrap_or("nom.db".to_string());

        let conn = rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::default())
            .expect("Could not open database");

        conn.execute_batch(
            "
        BEGIN;
        CREATE TABLE IF NOT EXISTS recipes (
            id INTEGER PRIMARY KEY,
            title TEXT,
            instructions TEXT
        );
        CREATE TABLE IF NOT EXISTS ingredients (
            id INTEGER PRIMARY KEY,
            recipe INTEGER,
            ingredient TEXT,
            FOREIGN KEY(recipe) REFERENCES recipes(id)
        );
        COMMIT;
    ",
        )
        .unwrap();

        tokio::sync::Mutex::new(conn)
    });

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
