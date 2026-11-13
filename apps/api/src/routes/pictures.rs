//! The shared picture library.

use axum::extract::{Multipart, Path, Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::extract::{AdminUser, AuthUser};
use crate::dto::{PictureReferences, PictureSummary};
use crate::error::{AppError, AppResult};
use crate::images;
use crate::models::Picture;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/pictures", get(list).post(upload))
        .route("/pictures/{id}", axum::routing::delete(remove))
        .route("/pictures/raw/{*key}", get(raw))
}

fn to_summary(picture: &Picture, state: &AppState) -> PictureSummary {
    PictureSummary {
        id: picture.id,
        url: state.storage.public_url(&picture.storage_key),
        width: picture.width,
        height: picture.height,
        created_at: picture.created_at,
    }
}

/// Every administrator sees every administrator's uploads; the library is
/// shared, not per-account.
async fn list(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<PictureSummary>>> {
    let pictures = sqlx::query_as::<_, Picture>(
        "SELECT * FROM pictures WHERE deleted_at IS NULL ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        pictures.iter().map(|p| to_summary(p, &state)).collect(),
    ))
}

async fn upload(
    State(state): State<AppState>,
    admin: AdminUser,
    mut multipart: Multipart,
) -> AppResult<Json<PictureSummary>> {
    let mut raw: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("malformed upload: {e}")))?
    {
        if field.name() == Some("file") {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| AppError::bad_request(format!("could not read the upload: {e}")))?;
            raw = Some(bytes.to_vec());
            break;
        }
    }

    let raw = raw.ok_or_else(|| AppError::bad_request("no file was uploaded"))?;

    // Validation, downscaling, and WebP transcoding all happen here, so what
    // reaches storage is always bounded and always one format.
    let processed = images::process(&raw)?;

    let id = Uuid::new_v4();
    let key = images::storage_key(id);

    state
        .storage
        .put(&key, processed.bytes, images::ProcessedImage::CONTENT_TYPE)
        .await?;

    let picture = sqlx::query_as::<_, Picture>(
        "INSERT INTO pictures (id, storage_key, width, height, uploaded_by)
         VALUES ($1, $2, $3, $4, $5) RETURNING *",
    )
    .bind(id)
    .bind(&key)
    .bind(processed.width)
    .bind(processed.height)
    .bind(admin.user.id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(to_summary(&picture, &state)))
}

#[derive(Debug, Deserialize)]
struct DeleteQuery {
    /// `false` (the default) soft-deletes: the picture leaves the picker but
    /// screens already using it are untouched.
    #[serde(default)]
    hard: bool,
}

async fn remove(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Query(query): Query<DeleteQuery>,
) -> AppResult<Response> {
    let picture = sqlx::query_as::<_, Picture>("SELECT * FROM pictures WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    if !query.hard {
        sqlx::query("UPDATE pictures SET deleted_at = now() WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .execute(&state.db)
            .await?;
        return Ok(Json(serde_json::json!({ "ok": true, "deleted": "soft" })).into_response());
    }

    // A hard delete is refused while any screen still renders the picture.
    let (screen_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*)::bigint FROM screens WHERE picture_id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    if screen_count > 0 {
        return Ok((
            axum::http::StatusCode::CONFLICT,
            Json(PictureReferences { screen_count }),
        )
            .into_response());
    }

    // Storage first: a dangling row is recoverable, a dangling object is not
    // discoverable.
    state.storage.delete(&picture.storage_key).await?;
    sqlx::query("DELETE FROM pictures WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true, "deleted": "hard" })).into_response())
}

/// Streams picture bytes when the bucket has no public domain.
///
/// Authenticated: pictures are the backgrounds of private screens. The session
/// cookie reaches this route from an `<img>` tag because the web and API hosts
/// share a registrable domain, which makes the request same-site.
async fn raw(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(key): Path<String>,
) -> AppResult<Response> {
    let bytes = state.storage.get(&key).await?;
    Ok((
        [
            (header::CONTENT_TYPE, images::ProcessedImage::CONTENT_TYPE),
            // Content at a given key never changes: a new upload is a new key.
            (header::CACHE_CONTROL, "private, max-age=31536000, immutable"),
        ],
        bytes,
    )
        .into_response())
}
