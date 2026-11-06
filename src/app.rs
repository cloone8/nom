use leptos::prelude::*;
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::hooks::use_url;
use leptos_router::{SsrMode, path};

use crate::pages::editrecipe::EditRecipePage;
use crate::pages::home::HomePage;
use crate::pages::newrecipe::NewRecipePage;
use crate::pages::recipe::RecipePage;
use crate::pages::trmnl::TrmnlPage;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <NavBar/>
            <main>
                // `SsrMode::Async`: the server waits for the data (local SQLite, so hardly any
                // wait) and sends each page complete, instead of a placeholder plus a swap script
                <Routes fallback=|| "Pagina niet gevonden.".into_view()>
                    <Route path=path!("/") view=HomePage ssr=SsrMode::Async/>
                    <Route path=path!("/trmnl") view=TrmnlPage ssr=SsrMode::Async/>
                    <Route path=path!("/recipe/:id") view=RecipePage ssr=SsrMode::Async/>
                    <Route path=path!("/edit/:id") view=EditRecipePage ssr=SsrMode::Async/>
                    <Route path=path!("/new") view=NewRecipePage ssr=SsrMode::Async/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn NavBar() -> impl IntoView {
    let url = use_url();

    view! {
        <Show when=move || url.read().path() != "/trmnl">
            <nav class="nom-navbar">
                // `exact`, or Home would count as the current page on every route
                <A attr:class="link-button" href="/" exact=true>"Home"</A>
                <A attr:class="link-button" href="/new">"Nieuw recept"</A>
                // A server route, not a page: `rel="external"` makes the router leave it to the browser
                <a class="link-button" href="/random" rel="external">"Random"</a>
                <ThemeButton/>
            </nav>
        </Show>
    }
}

/// Cycles the colour scheme: Auto → Licht → Donker. Plain JS, so it works before the WASM has
/// loaded. The CSS shows the label of the current scheme, so it's right from the first paint.
#[component]
fn ThemeButton() -> impl IntoView {
    view! {
        <button
            class="link-button theme"
            title="Kleurenschema: Auto (volgt het systeem), Licht of Donker"
            onclick={include_str!("js/theme_click.js")}
        >
            <span class="auto">"🌓 Auto"</span>
            <span class="light">"☀️ Licht"</span>
            <span class="dark">"🌙 Donker"</span>
        </button>
    }
}
