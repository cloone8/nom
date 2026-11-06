//! Photos in recipes: uploaded from the recipe form, stored in the database, served from
//! `/images/<id>` and placed in the Markdown as `![](/images/<id>)`.

use leptos::prelude::*;
use leptos::server_fn::codec::{MultipartData, MultipartFormData};

/// Larger uploads are refused. Photos are shrunk in the browser first, so this only matters
/// when that fails and the original is sent.
#[cfg(feature = "ssr")]
const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024;

/// Stores the image in the multipart field (the first one) and returns its id.
#[server(input = MultipartFormData)]
pub async fn upload_image(data: MultipartData) -> Result<i64, ServerFnError> {
    use crate::db;

    let mut data = data.into_inner().expect("always Some on the server");
    let mut field = data
        .next_field()
        .await?
        .ok_or_else(|| ServerFnError::new("Geen foto ontvangen"))?;

    let mime = field
        .content_type()
        .map(|mime| mime.to_string())
        .filter(|mime| mime.starts_with("image/"))
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

    use crate::db;

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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Shrinks `file`, uploads it, and inserts its Markdown at the cursor in `textarea`. Returns a
/// message to show if that fails.
#[cfg(feature = "hydrate")]
pub async fn upload_and_insert(
    file: web_sys::File,
    textarea: web_sys::HtmlTextAreaElement,
) -> Result<(), String> {
    use wasm_bindgen::JsCast;

    // A photo the browser can't decode or encode (an SVG, say) goes up as it is
    let blob = shrink(&file)
        .await
        .unwrap_or_else(|_| file.clone().unchecked_into());

    let form = web_sys::FormData::new().map_err(|_| "Kan de foto niet versturen")?;
    form.append_with_blob_and_filename("image", &blob, &file.name())
        .map_err(|_| "Kan de foto niet versturen")?;

    let id = upload_image(form.into())
        .await
        .map_err(|error| match error {
            ServerFnError::ServerError(message) => message,
            other => other.to_string(),
        })?;

    // Positions are in UTF-16 units, like `selectionStart`
    let text: Vec<u16> = textarea.value().encode_utf16().collect();
    let end = text.len() as u32;
    let start = textarea.selection_start().ok().flatten().unwrap_or(end);
    let stop = textarea.selection_end().ok().flatten().unwrap_or(end);

    // On a line of its own, so it doesn't stick to the text around the cursor
    let newline = u16::from(b'\n');
    let before = if start > 0 && text[start as usize - 1] != newline {
        "\n"
    } else {
        ""
    };
    let after = if stop < end && text[stop as usize] != newline {
        "\n"
    } else {
        ""
    };

    textarea
        .set_range_text_with_start_and_end_and_mode(
            &format!("{before}![](/images/{id}){after}"),
            start,
            stop,
            // Puts the cursor after the inserted text
            "end",
        )
        .map_err(|_| format!("Foto geüpload, maar invoegen lukte niet: ![](/images/{id})"))?;

    Ok(())
}

/// Longest side of an uploaded photo, in pixels.
#[cfg(feature = "hydrate")]
const MAX_SIDE: f64 = 1600.0;

/// Scales the photo down to at most [`MAX_SIDE`] and re-encodes it as JPEG.
#[cfg(feature = "hydrate")]
async fn shrink(file: &web_sys::File) -> Result<web_sys::Blob, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::OffscreenCanvasRenderingContext2d as Context2d;
    use web_sys::{ImageBitmap, ImageEncodeOptions, OffscreenCanvas};

    // Applies the photo's EXIF rotation, so the result is upright
    let bitmap: ImageBitmap = JsFuture::from(window().create_image_bitmap_with_blob(file)?)
        .await?
        .unchecked_into();

    let (width, height) = (bitmap.width() as f64, bitmap.height() as f64);
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
