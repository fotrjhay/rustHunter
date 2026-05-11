use axum::{extract::Query, extract::State, Json};
use tracing::{debug, info};

use crate::errors::AppError;
use crate::models::{JobListing, SearchQuery};
use crate::AppState;

pub async fn search_jobs(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<JobListing>>, AppError> {
    let query = params.search_text().unwrap_or_default().trim().to_owned();
    let page = params.page.unwrap_or(1);
    let include_description = params.include_description.unwrap_or(true);

    info!(query = %query, page, include_description, "received search request");

    if query.is_empty() {
        return Err(AppError::EmptyQuery);
    }

    if page == 0 {
        return Err(AppError::InvalidPage);
    }

    debug!(
        area_code = state.config.area_code,
        items_per_page = state.config.items_per_page,
        include_description,
        "using search configuration"
    );

    let results = state
        .job_search
        .search(&query, page, include_description)
        .await?;
    info!(count = results.len(), "returning search results");

    Ok(Json(results))
}
