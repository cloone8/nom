use leptos::ev::SubmitEvent;
use leptos::logging::log;
use leptos::reactive::spawn_local;
use leptos::{html, prelude::*};
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::hooks::use_params;
use leptos_router::{NavigateOptions, path};
use leptos_router::{
    components::{Route, Router, Routes},
    params::Params,
};

use crate::recipe::{
    ListedRecipe, RawRecipe, Recipe, RecipeComponent, get_recipe, list_recipes, new_recipe,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
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

#[server]
pub async fn log_on_server(msg: String) -> Result<(), ServerFnError> {
    log!("Message: {msg}");
    Ok(())
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/nom_front.css"/>

        // sets the document title
        <Title text="NomNomNom"/>

        // content for this welcome page
        <Router>
            <nav>
                <NavBar/>
            </nav>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/recipe/:id") view=RecipePage/>
                    <Route path=path!("/new") view=NewRecipePage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn NavBar() -> impl IntoView {
    view! {
        <a href="/">"Home"</a><br/>
        <a href="/new">"Nieuw recept"</a><br/>
        <a href="/recipe/test">"Test recept"</a><br/>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    async fn fetch() -> Vec<ListedRecipe> {
        list_recipes().await.unwrap()
    }

    view! {
        <h1>"Het NomNomNom Receptenboek"</h1>
        <Await
            future=fetch()
            let:recipes
        >
            <ul>
                {
                    recipes.iter().map(|rp|{
                        let url = format!("/recipe/{}", rp.id);

                        view! {
                            <li><a href={url}>{rp.title.clone()}</a></li>
                        }
                    }).collect_view()
                }
            </ul>
        </Await>
    }
}

#[derive(Debug, Params, PartialEq)]
struct RecipeArgs {
    id: Option<String>,
}

#[component]
fn RecipePage() -> impl IntoView {
    let id = move || {
        use_params::<RecipeArgs>()
            .read()
            .as_ref()
            .ok()
            .and_then(|params| params.id.clone())
            .unwrap()
    };

    let recipe_resource = Resource::new(id, async |id| {
        if id == "test" {
            Some(Recipe::test())
        } else {
            get_recipe(id.parse().unwrap()).await.unwrap()
        }
    });

    let render_recipe = move || {
        recipe_resource.get().map(|recipe| match recipe {
            Some(recipe) => view! {<RecipeComponent recipe={recipe}/> }.into_any(),
            None => view! { <h2>"Onbekend recept"</h2>}.into_any(),
        })
    };

    view! {
        <h1>"Het NomNomNom Receptenboek " {id}</h1>
        <Suspense fallback=move || view!{ <p>"Recept aan het laden..."</p>}>
            {render_recipe}
        </Suspense>
    }
}

#[component]
fn NewRecipePage() -> impl IntoView {
    let title_elem: NodeRef<html::Input> = NodeRef::new();
    let ingredient_elem: NodeRef<html::Textarea> = NodeRef::new();
    let instruction_elem: NodeRef<html::Textarea> = NodeRef::new();

    let on_submit = move |ev: SubmitEvent| {
        // stop the page from reloading!
        ev.prevent_default();

        let title = title_elem.get().unwrap().value();
        let ingredients = ingredient_elem.get().unwrap().value();
        let instructions = instruction_elem.get().unwrap().value();

        spawn_local(async {
            new_recipe(RawRecipe {
                title,
                ingredients,
                instructions,
            })
            .await
            .unwrap();

            let navigate = leptos_router::hooks::use_navigate();

            navigate("/", NavigateOptions::default());
        });
    };

    view! {
        <h1>"Nieuw recept"</h1>
        <form on:submit=on_submit>
            <h3>Titel</h3>
            <input type="text" placeholder="Titel" node_ref=title_elem/>
            <br/>
            <h3>Ingredienten</h3>
            <textarea placeholder="Ingredienten" node_ref=ingredient_elem/>
            <br/>
            <h3>Instructies</h3>
            <textarea placeholder="Instructies" node_ref=instruction_elem/>
            <br/>
            <input type="submit" value="Maak"/>
        </form>
    }
}
