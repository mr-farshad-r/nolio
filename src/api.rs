use crate::db::Db;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
pub struct PageInput {
    pub parent_id: Option<String>,
    pub title: Option<String>,
    pub icon: Option<String>,
}

#[derive(Serialize)]
pub struct Page {
    pub id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub icon: Option<String>,
}

fn row_to_page(row: &rusqlite::Row) -> rusqlite::Result<Page> {
    Ok(Page {
        id: row.get("id")?,
        parent_id: row.get("parent_id")?,
        title: row.get("title")?,
        icon: row.get("icon")?,
    })
}

pub fn router(db: Db) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/pages", get(list_pages).post(create_page))
        .route("/api/pages/:id", get(get_page).patch(update_page).delete(delete_page))
        .route("/api/blocks/:page_id", get(list_blocks).post(create_block))
        .route("/api/blocks/block/:id", get(get_block).patch(update_block).delete(delete_block))
        .with_state(db)
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../web/index.html"))
}

// ---- pages ----

async fn list_pages(State(db): State<Db>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.lock().await;
    let mut stmt = conn
        .prepare("SELECT id, parent_id, title, icon FROM page WHERE deleted = 0 ORDER BY updated_at DESC")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let pages = stmt
        .query_map([], row_to_page)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!(pages)))
}

async fn create_page(
    State(db): State<Db>,
    Json(input): Json<PageInput>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let conn = db.lock().await;
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO page (id, workspace_id, parent_id, title, icon) VALUES (?1, 'default', ?2, ?3, ?4)",
        rusqlite::params![id, input.parent_id, input.title.unwrap_or_default(), input.icon],
    )
    .map_err(|e| { eprintln!("create_page error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn get_page(
    State(db): State<Db>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.lock().await;
    let mut stmt = conn
        .prepare("SELECT id, parent_id, title, icon FROM page WHERE id = ?1 AND deleted = 0")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rows = stmt
        .query_map([id], row_to_page)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match rows.next() {
        Some(Ok(page)) => Ok(Json(json!(page))),
        _ => Err(StatusCode::NOT_FOUND),
    }
}

async fn update_page(
    State(db): State<Db>,
    Path(id): Path<String>,
    Json(input): Json<PageInput>,
) -> Result<StatusCode, StatusCode> {
    let conn = db.lock().await;
    conn.execute(
        "UPDATE page SET title = COALESCE(?2, title), icon = COALESCE(?3, icon), updated_at = datetime('now') WHERE id = ?1",
        rusqlite::params![id, input.title, input.icon],
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_page(
    State(db): State<Db>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let conn = db.lock().await;
    conn.execute("UPDATE page SET deleted = 1 WHERE id = ?1", [id])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- blocks ----

async fn list_blocks(
    State(db): State<Db>,
    Path(page_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.lock().await;
    let mut stmt = conn
        .prepare("SELECT id, type, content, ord FROM block WHERE page_id = ?1 AND deleted = 0 ORDER BY ord")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let blocks: Vec<serde_json::Value> = stmt
        .query_map([page_id], |row| {
            let content: String = row.get(2)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "type": row.get::<_, String>(1)?,
                "content": serde_json::from_str::<serde_json::Value>(&content).unwrap_or(json!({})),
                "ord": row.get::<_, f64>(3)?,
            }))
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!(blocks)))
}

async fn create_block(
    State(db): State<Db>,
    Path(page_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let conn = db.lock().await;
    let id = uuid::Uuid::new_v4().to_string();
    let btype = body.get("type").and_then(|t| t.as_str()).unwrap_or("paragraph");
    let content = body.get("content").cloned().unwrap_or(json!({}));
    conn.execute(
        "INSERT INTO block (id, page_id, type, content) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, page_id, btype, content.to_string()],
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn get_block(
    State(db): State<Db>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.lock().await;
    let mut stmt = conn
        .prepare("SELECT id, page_id, parent_id, type, content, ord FROM block WHERE id = ?1 AND deleted = 0")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rows = stmt
        .query_map([id], |row| {
            let content: String = row.get(4)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "page_id": row.get::<_, String>(1)?,
                "parent_id": row.get::<_, Option<String>>(2)?,
                "type": row.get::<_, String>(3)?,
                "content": serde_json::from_str::<serde_json::Value>(&content).unwrap_or(json!({})),
                "ord": row.get::<_, f64>(5)?,
            }))
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match rows.next() {
        Some(Ok(block)) => Ok(Json(block)),
        _ => Err(StatusCode::NOT_FOUND),
    }
}

async fn update_block(
    State(db): State<Db>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    let conn = db.lock().await;
    if let Some(content) = body.get("content") {
        conn.execute(
            "UPDATE block SET content = ?2 WHERE id = ?1",
            rusqlite::params![id, content.to_string()],
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_block(State(db): State<Db>, Path(id): Path<String>) -> Result<StatusCode, StatusCode> {
    let conn = db.lock().await;
    conn.execute("UPDATE block SET deleted = 1 WHERE id = ?1", [id])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
