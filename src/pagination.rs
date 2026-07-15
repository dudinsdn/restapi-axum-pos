use axum::{
    Json,
    http::HeaderValue,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// Used when the client doesn't send `?limit=`.
pub const DEFAULT_LIMIT: usize = 50;
/// Hard ceiling on `?limit=`, regardless of what the client asks for — so a
/// single request can't force the server to serialize an unbounded amount
/// of data.
pub const MAX_LIMIT: usize = 200;

/// Query params accepted by every list endpoint (`?limit=&offset=`).
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl PaginationQuery {
    /// Clamps `limit` to `[1, MAX_LIMIT]` (defaulting to `DEFAULT_LIMIT` if
    /// unset) and treats a missing `offset` as `0`. Never errors — an
    /// out-of-range value is silently clamped rather than rejected, since a
    /// client asking for `limit=999999` almost certainly just wants "as much
    /// as you'll give me", not a `400`.
    fn normalized(&self) -> (usize, usize) {
        let limit = self.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let offset = self.offset.unwrap_or(0);
        (limit, offset)
    }
}

/// Slices `items` according to `query`, then wraps the page as a JSON array
/// response carrying an `X-Total-Count` header with the total item count
/// (before slicing).
///
/// The body stays a plain JSON array — deliberately NOT wrapped in an
/// envelope like `{"data": [...], "total": N}` — so existing clients that
/// already expect a plain array from these endpoints keep working
/// unchanged; pagination is purely additive via query params + the response
/// header. This is a stopgap: it still fetches the FULL collection from the
/// repository and paginates in memory, so it doesn't help until the
/// storage layer itself supports paginated queries (e.g. `LIMIT`/`OFFSET`
/// in a real database).
pub fn paginated_response<T: Serialize>(
    items: Vec<T>,
    query: &PaginationQuery,
) -> Response {
    let total = items.len();
    let (limit, offset) = query.normalized();
    let page: Vec<T> = items.into_iter().skip(offset).take(limit).collect();

    let mut response = Json(page).into_response();
    if let Ok(value) = HeaderValue::from_str(&total.to_string()) {
        response.headers_mut().insert("x-total-count", value);
    }
    response
}
