use std::sync::OnceLock;

use leptos::prelude::*;
use leptos_meta::{MetaTags, Title, provide_meta_context};
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::hooks::use_url;
use leptos_router::path;

use crate::pages::editrecipe::EditRecipePage;
use crate::pages::home::HomePage;
use crate::pages::newrecipe::NewRecipePage;
use crate::pages::recipe::RecipePage;
use crate::pages::trmnl::TrmnlPage;

/// The compiled stylesheet, inlined into every page so it costs no extra request. Set at
/// startup in production only: in development, pages link to the file instead, so
/// `cargo leptos watch` can swap in CSS changes live.
static INLINE_CSS: OnceLock<String> = OnceLock::new();

/// Reads the stylesheet cargo-leptos compiled from `style/main.scss` and inlines it from now on.
#[cfg(feature = "ssr")]
pub fn inline_css(options: &LeptosOptions) -> Result<(), String> {
    let path = std::path::Path::new(&*options.site_root)
        .join(&*options.site_pkg_dir)
        .join(format!("{}.css", options.output_name));

    let css = std::fs::read_to_string(&path)
        .map_err(|e| format!("can't read the stylesheet {}: {e}", path.display()))?;

    INLINE_CSS
        .set(css)
        .map_err(|_| "inline_css called twice".to_string())
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    // Only rendered on the server, and outside the hydrated body, so the two forms can differ
    let stylesheet = match INLINE_CSS.get() {
        // `inner_html`, because text children would be HTML-escaped (`>` in selectors)
        Some(css) => view! { <style inner_html=css.as_str()></style> }.into_any(),
        None => {
            let href = format!("/{}/{}.css", options.site_pkg_dir, options.output_name);
            // `id="leptos"` is how `cargo leptos watch` finds the stylesheet to reload
            view! { <link id="leptos" rel="stylesheet" href=href/> }.into_any()
        }
    };

    view! {
        <!DOCTYPE html>
        <html lang="nl">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                {stylesheet}
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Title text="NomNomNom"/>

        <Router>
            <NavBar/>
            <main>
                <Routes fallback=|| "Pagina niet gevonden.".into_view()>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/trmnl") view=TrmnlPage/>
                    <Route path=path!("/recipe/:id") view=RecipePage/>
                    <Route path=path!("/edit/:id") view=EditRecipePage/>
                    <Route path=path!("/new") view=NewRecipePage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn NavBar() -> impl IntoView {
    let url = use_url();
    let url_is_trmnl = move || url.get().path() == "/trmnl";

    view! {
        {
            move || if url_is_trmnl() {
                ().into_any()
            } else {
                view! {
                    <nav class="nom-navbar">
                        // `exact`, or Home would count as the current page on every route
                        <A attr:class="link-button" href="/" exact=true>"Home"</A>
                        <A attr:class="link-button" href="/new">"Nieuw recept"</A>
                        // A server route, not a page: `rel="external"` makes the router leave it to the browser
                        <a class="link-button" href="/random" rel="external">"Random"</a>
                    </nav>
                }.into_any()
            }
        }
    }
}
