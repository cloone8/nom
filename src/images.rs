//! Photos in recipes: uploaded from the recipe form as drafts, which become the recipe's when it
//! is saved (`db::set_recipe_photos`). Served from `/images/<id>`; the recipe page shows the
//! first as the header photo and the rest in a gallery.

use leptos::prelude::*;
use leptos::server_fn::codec::{MultipartData, MultipartFormData};

#[cfg(feature = "ssr")]
use crate::db;

/// Larger uploads are refused. Photos are shrunk in the browser first, so this only matters
/// when that fails and the original is sent.
#[cfg(feature = "ssr")]
const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024;

/// Stores the image in the multipart field (the first one) and returns its id.
#[server(input = MultipartFormData)]
pub async fn upload_image(data: MultipartData) -> Result<i64, ServerFnError> {
    let mut data = data.into_inner().expect("always Some on the server");
    let mut field = data
        .next_field()
        .await?
        .ok_or_else(|| ServerFnError::new("Geen foto ontvangen"))?;

    let mime = field
        .content_type()
        // Without parameters such as `; charset=...`
        .map(|mime| mime.essence_str().to_string())
        .filter(|mime| mime.starts_with("image/") && mime.len() > "image/".len())
        .ok_or_else(|| ServerFnError::new("Dat bestand is geen afbeelding"))?;

    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await? {
        bytes.extend_from_slice(&chunk);

        if bytes.len() > MAX_UPLOAD_BYTES {
            return Err(ServerFnError::new("De foto is groter dan 20 MB"));
        }
    }

    if bytes.is_empty() {
        return Err(ServerFnError::new("Het bestand is leeg"));
    }

    Ok(db::insert_image(&db::connection(), &mime, &bytes)?)
}

/// Handler for `GET /images/{id}`.
#[cfg(feature = "ssr")]
pub async fn serve_image(
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::http::header::{
        CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS,
    };
    use axum::response::IntoResponse;

    match db::get_image(&db::connection(), id) {
        Ok(Some((mime, data))) => (
            [(CONTENT_TYPE, mime)],
            [
                // An id never gets another image, so the browser may keep it forever.
                // `private`: everything is behind auth.
                (CACHE_CONTROL, "private, max-age=31536000, immutable"),
                // Any image type is accepted. If an SVG with a script is opened directly,
                // `sandbox` keeps that script away from the site.
                (CONTENT_SECURITY_POLICY, "sandbox"),
                (X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            data,
        )
            .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            leptos::logging::log!("Serving image {id} failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Shrinks `file` and uploads it as a draft. Returns its id, or a message to show. Browser
/// only.
pub async fn upload(file: web_sys::File) -> Result<i64, String> {
    use wasm_bindgen::JsCast;

    // A photo the browser can't decode or encode (an SVG, say) goes up as it is
    let blob = shrink(&file)
        .await
        .unwrap_or_else(|_| file.clone().unchecked_into());

    let form = web_sys::FormData::new().map_err(|_| "Kan de foto niet versturen")?;
    form.append_with_blob_and_filename("image", &blob, &file.name())
        .map_err(|_| "Kan de foto niet versturen")?;

    upload_image(form.into())
        .await
        .map_err(crate::pages::error_message)
}

/// Removes `![](/images/<id>)` from the text, including the line it had to itself (as the
/// earlier "In tekst" button placed it). Browser only.
pub fn remove_from_text(textarea: &web_sys::HtmlTextAreaElement, id: i64) {
    let image = format!("![](/images/{id})");
    let value = textarea.value();

    let cleaned = value
        .replace(&format!("\n{image}\n"), "\n")
        .replace(&format!("\n{image}"), "")
        .replace(&format!("{image}\n"), "")
        .replace(&image, "");

    if cleaned != value {
        textarea.set_value(&cleaned);
    }
}

/// Longest side of an uploaded photo, in pixels.
const MAX_SIDE: f64 = 1600.0;

/// Scales the photo down to at most [`MAX_SIDE`] and re-encodes it as JPEG.
async fn shrink(file: &web_sys::File) -> Result<web_sys::Blob, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::OffscreenCanvasRenderingContext2d as Context2d;
    use web_sys::{ImageBitmap, ImageEncodeOptions, OffscreenCanvas};

    // Applies the photo's EXIF rotation, so the result is upright
    let bitmap: ImageBitmap = JsFuture::from(window().create_image_bitmap_with_blob(file)?)
        .await?
        .unchecked_into();

    let (width, height) = (f64::from(bitmap.width()), f64::from(bitmap.height()));
    let scale = (MAX_SIDE / width.max(height)).min(1.0);
    let (width, height) = ((width * scale).round(), (height * scale).round());

    let canvas = OffscreenCanvas::new(width as u32, height as u32)?;
    let context: Context2d = canvas
        .get_context("2d")?
        .ok_or("no 2d canvas context")?
        .unchecked_into();

    // JPEG has no transparency: without this, transparent parts would turn black
    context.set_fill_style_str("white");
    context.fill_rect(0.0, 0.0, width, height);
    context.draw_image_with_image_bitmap_and_dw_and_dh(&bitmap, 0.0, 0.0, width, height)?;
    bitmap.close();

    let options = ImageEncodeOptions::new();
    options.set_type("image/jpeg");
    options.set_quality(0.85);

    Ok(
        JsFuture::from(canvas.convert_to_blob_with_options(&options)?)
            .await?
            .unchecked_into(),
    )
}
