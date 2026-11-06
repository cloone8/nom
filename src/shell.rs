//! The HTML document around the app. Server only.

use std::sync::OnceLock;

use leptos::prelude::*;

use crate::app::App;

/// 🍲 as an SVG, inline so the icon costs no request.
const FAVICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'%3E%3Ctext y='.9em' font-size='90'%3E%F0%9F%8D%B2%3C/text%3E%3C/svg%3E";

/// How pages get the stylesheet cargo-leptos compiles from `style/main.scss`.
enum Stylesheet {
    /// The CSS itself, inlined into every page so it costs no request (production).
    Inline(String),
    /// A link to the file, which `cargo leptos watch` can swap when the CSS changes
    /// (development).
    Link(String),
}

static STYLESHEET: OnceLock<Stylesheet> = OnceLock::new();

/// Finds the compiled stylesheet, and in production reads it for inlining. Call once at startup.
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

pub fn shell(options: LeptosOptions) -> impl IntoView {
    let stylesheet = match STYLESHEET.get().expect("load_stylesheet wasn't called") {
        // `inner_html`, because text children would be HTML-escaped (`>` in selectors)
        Stylesheet::Inline(css) => view! { <style inner_html=css.as_str()></style> }.into_any(),
        // `id="leptos"` is what `cargo leptos watch` looks for
        Stylesheet::Link(href) => {
            view! { <link id="leptos" rel="stylesheet" href=href.as_str()/> }.into_any()
        }
    };

    view! {
        <!DOCTYPE html>
        <html lang="nl">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>"NomNomNom"</title>
                <link rel="icon" href=FAVICON/>
                // Plain JS, because the WASM loads too late for that
                <script inner_html={include_str!("js/theme_init.js")}></script>
                {stylesheet}
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}
