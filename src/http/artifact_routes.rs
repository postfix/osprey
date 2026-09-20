//! `GET|HEAD /npm/artifacts/{reference_id}/{filename}` and its PyPI twin (SPEC §11).
//! The handler validates, delegates, and maps errors — every policy decision belongs
//! to `artifacts::serve_artifact`.
//!
//! The ecosystem is in the route rather than in a path capture, so it is a fact about
//! which registration matched and not a string a client chose.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, header};
use axum::response::{IntoResponse, Response};

use crate::App;
use crate::artifacts::{self};
use crate::http::error::ApiError;
use crate::policy::Ecosystem;
use crate::store::rows::ReferenceId;

pub async fn serve_npm(
    state: State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
    path: Path<(String, String)>,
) -> Result<Response, ApiError> {
    serve(Ecosystem::Npm, state, method, headers, path).await
}

pub async fn serve_pypi(
    state: State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
    path: Path<(String, String)>,
) -> Result<Response, ApiError> {
    serve(Ecosystem::PyPi, state, method, headers, path).await
}

async fn serve(
    ecosystem: Ecosystem,
    State(app): State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
    Path((reference_id, filename)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    // SPEC §10: "reject overload instead of allowing unbounded waiters or tasks."
    // The permit is handed to the response below, so it is held for as long as bytes
    // are still going out and not merely until this function returns.
    let permit = app.limits.active_permit().ok_or(ApiError::Overloaded)?;

    // A reference id is 64 hexadecimal characters and nothing else. SPEC §9: it is a
    // lookup key, so an unparseable one is invalid input rather than a miss.
    let id = ReferenceId::parse_hex(&reference_id).map_err(|err| {
        tracing::debug!(error = %err, "refused an artifact reference id");
        ApiError::InvalidInput("a reference id is 64 hexadecimal characters")
    })?;

    let range = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());

    let response =
        artifacts::serve_artifact(&app, ecosystem, id, &filename, &method, range).await?;
    Ok(response.under(&app.limits, permit).into_response())
}
