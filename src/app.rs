use leptos::prelude::*;
use leptos_meta::{MetaTags, Title, provide_meta_context};
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::hooks::use_url;
use leptos_router::{SsrMode, path};

use crate::pages::editrecipe::EditRecipePage;
use crate::pages::home::HomePage;
use crate::pages::newrecipe::NewRecipePage;
use crate::pages::recipe::RecipePage;
use crate::pages::trmnl::TrmnlPage;

/// 🍲 as an SVG, inline so the icon costs no request.
const FAVICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'%3E%3Ctext y='.9em' font-size='90'%3E%F0%9F%8D%B2%3C/text%3E%3C/svg%3E";

/// How pages get the stylesheet cargo-leptos compiles from `style/main.scss`.
#[cfg(feature = "ssr")]
enum Stylesheet {
    /// The CSS itself, inlined into every page so it costs no request (production).
    Inline(String),
    /// A link to the file, which `cargo leptos watch` can swap when the CSS changes
    /// (development).
    Link(String),
}

#[cfg(feature = "ssr")]
static STYLESHEET: std::sync::OnceLock<Stylesheet> = std::sync::OnceLock::new();

/// Finds the compiled stylesheet, and in production reads it for inlining. Call once at startup.
#[cfg(feature = "ssr")]
pub fn load_stylesheet(options: &LeptosOptions) -> Result<(), String> {
    let file_name = format!("{}.css", hashed_output_name(options, "css")?);

    let stylesheet = if options.env == Env::PROD {
        let path = std::path::Path::new(&*options.site_root)
            .join(&*options.site_pkg_dir)
            .join(&file_name);
        let css = std::fs::read_to_string(&path)
            .map_err(|e| format!("can't read the stylesheet {}: {e}", path.display()))?;

        Stylesheet::Inline(css)
    } else {
        Stylesheet::Link(format!("/{}/{file_name}", options.site_pkg_dir))
    };

    STYLESHEET
        .set(stylesheet)
        .map_err(|_| "load_stylesheet called twice".to_string())
}

/// `output_name`, plus `.<hash>` for `kind` (`js`, `wasm` or `css`) if the file names are hashed.
#[cfg(feature = "ssr")]
fn hashed_output_name(options: &LeptosOptions, kind: &str) -> Result<String, String> {
    if !options.hash_files {
        return Ok(options.output_name.to_string());
    }

    // Next to the server binary, where `HydrationScripts` reads it too
    let path = std::env::current_exe()
        .map_err(|e| format!("can't find the server binary: {e}"))?
        .with_file_name(&*options.hash_file);
    let hashes = std::fs::read_to_string(&path)
        .map_err(|e| format!("can't read {}: {e}", path.display()))?;

    // Lines like `css: agVEcJizFfJ5rzjYV8ZKGw`
    hashes
        .lines()
        .find_map(|line| {
            let (file, hash) = line.split_once(':')?;
            (file.trim() == kind).then(|| format!("{}.{}", options.output_name, hash.trim()))
        })
        .ok_or_else(|| format!("{} has no {kind} hash", path.display()))
}

/// Only rendered on the server, and outside the hydrated body, so production and development
/// can differ.
fn stylesheet() -> AnyView {
    #[cfg(feature = "ssr")]
    match STYLESHEET.get() {
        // `inner_html`, because text children would be HTML-escaped (`>` in selectors)
        Some(Stylesheet::Inline(css)) => {
            return view! { <style inner_html=css.as_str()></style> }.into_any();
        }
        // `id="leptos"` is what `cargo leptos watch` looks for
        Some(Stylesheet::Link(href)) => {
            return view! { <link id="leptos" rel="stylesheet" href=href.as_str()/> }.into_any();
        }
        None => {}
    }

    ().into_any()
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="nl">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <link rel="icon" href=FAVICON/>
                {stylesheet()}
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
