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

/// The `localStorage` key for the colour scheme chosen with the theme button.
const THEME_KEY: &str = "nom-theme";

/// Applies the saved colour scheme before the page is drawn, so a dark choice doesn't flash
/// light first. Plain JS, because the WASM loads too late for that. Uses [`THEME_KEY`].
const THEME_SCRIPT: &str = r#"try { const t = localStorage.getItem("nom-theme"); if (t === "light" || t === "dark") document.documentElement.dataset.theme = t } catch {}"#;

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
                <script inner_html=THEME_SCRIPT></script>
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
                        <ThemeButton/>
                    </nav>
                }.into_any()
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Theme {
    /// Follows the system setting
    Auto,
    Light,
    Dark,
}

impl Theme {
    fn next(self) -> Self {
        match self {
            Theme::Auto => Theme::Light,
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Auto,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Theme::Auto => "🌓 Auto",
            Theme::Light => "☀️ Licht",
            Theme::Dark => "🌙 Donker",
        }
    }

    /// The value in `localStorage` and in `<html data-theme>`; Auto has none.
    fn value(self) -> Option<&'static str> {
        match self {
            Theme::Auto => None,
            Theme::Light => Some("light"),
            Theme::Dark => Some("dark"),
        }
    }
}

/// Cycles the colour scheme: Auto → Licht → Donker. The choice is remembered per browser.
#[component]
fn ThemeButton() -> impl IntoView {
    // Auto on the server and while hydrating, so both render the same. Once running, show the
    // saved choice (which `THEME_SCRIPT` already applied). Effects never run on the server.
    let theme = RwSignal::new(Theme::Auto);
    Effect::new(move |_| {
        let saved = window()
            .local_storage()
            .ok()
            .flatten()
            .and_then(|storage| storage.get_item(THEME_KEY).ok().flatten());

        theme.set(match saved.as_deref() {
            Some("light") => Theme::Light,
            Some("dark") => Theme::Dark,
            _ => Theme::Auto,
        });
    });

    let next = move |_| {
        let next = theme.get_untracked().next();
        theme.set(next);

        // A failure only means the choice isn't remembered (private browsing) or shown, so
        // it's ignored
        if let Some(storage) = window().local_storage().ok().flatten() {
            let _ = match next.value() {
                Some(value) => storage.set_item(THEME_KEY, value),
                None => storage.remove_item(THEME_KEY),
            };
        }

        if let Some(root) = document().document_element() {
            let _ = match next.value() {
                Some(value) => root.set_attribute("data-theme", value),
                None => root.remove_attribute("data-theme"),
            };
        }
    };

    view! {
        <button
            class="link-button theme"
            title="Kleurenschema: Auto (volgt het systeem), Licht of Donker"
            on:click=next
        >
            {move || theme.get().label()}
        </button>
    }
}
