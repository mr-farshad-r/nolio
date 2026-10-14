use crate::db::Db;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
pub struct PageInput {
    pub parent_id: Option<String>,
    pub title: Option<String>,
    pub icon: Option<String>,
    pub favorite: Option<bool>,
}

#[derive(Serialize)]
pub struct Page {
    pub id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub icon: Option<String>,
    pub favorite: bool,
}

fn row_to_page(row: &rusqlite::Row) -> rusqlite::Result<Page> {
    Ok(Page {
        id: row.get("id")?,
        parent_id: row.get("parent_id")?,
        title: row.get("title")?,
        icon: row.get("icon")?,
        favorite: row.get::<_, i64>("favorite")? == 1,
    })
}

pub fn router(db: Db) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/app.css", get(serve_css))
        .route("/app.js", get(serve_js))
        .route("/api/pages", get(list_pages).post(create_page))
        .route("/api/pages/:id", get(get_page).patch(update_page).delete(delete_page))
        .route("/api/blocks/:page_id", get(list_blocks).post(create_block))
        .route("/api/blocks/block/:id", get(get_block).patch(update_block).delete(delete_block))
        .route("/api/blocks/block/:id/move", post(move_block))
        // databases
        .route("/api/databases", get(list_databases).post(create_database))
        .route("/api/databases/:id", get(get_database).delete(delete_database))
        .route("/api/databases/:id/rows", get(list_rows).post(create_row))
        .route("/api/databases/:id/props", post(add_property))
        .route("/api/databases/:id/rows/:page_id/props/:prop_id", axum::routing::put(set_property))
        .with_state(db)
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../web/index.html"))
}

async fn serve_css() -> (StatusCode, [(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../web/app.css"),
    )
}

async fn serve_js() -> (StatusCode, [(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        include_str!("../web/app.js"),
    )
}

// ---- pages ----

async fn list_pages(State(db): State<Db>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.lock().await;
    let mut stmt = conn
        .prepare("SELECT id, parent_id, title, icon, favorite FROM page WHERE deleted = 0 ORDER BY updated_at DESC")
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
        .prepare("SELECT id, parent_id, title, icon, favorite FROM page WHERE id = ?1 AND deleted = 0")
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
    let fav = input.favorite.map(|f| if f { 1 } else { 0 });
    conn.execute(
        "UPDATE page SET title = COALESCE(?2, title), icon = COALESCE(?3, icon),
         favorite = COALESCE(?4, favorite), updated_at = datetime('now') WHERE id = ?1",
        rusqlite::params![id, input.title, input.icon, fav],
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
        .prepare("SELECT id, parent_id, type, content, ord FROM block WHERE page_id = ?1 AND deleted = 0 ORDER BY ord")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let blocks: Vec<serde_json::Value> = stmt
        .query_map([page_id], |row| {
            let content: String = row.get(3)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "parent_id": row.get::<_, Option<String>>(1)?,
                "type": row.get::<_, String>(2)?,
                "content": serde_json::from_str::<serde_json::Value>(&content).unwrap_or(json!({})),
                "ord": row.get::<_, f64>(4)?,
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
    let parent_id = body.get("parent_id").and_then(|p| p.as_str()).map(|s| s.to_string());
    conn.execute(
        "INSERT INTO block (id, page_id, parent_id, type, content) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, page_id, parent_id, btype, content.to_string()],
    )
    .map_err(|e| { eprintln!("create_block error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

/// Move a block: change its parent (nesting) and/or position (ord).
/// Rejects ancestry cycles by walking up from the proposed parent.
async fn move_block(
    State(db): State<Db>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    let conn = db.lock().await;
    let new_parent: Option<String> = body
        .get("parent_id")
        .and_then(|p| p.as_str())
        .map(|s| s.to_string());

    if let Some(ref p) = new_parent {
        let mut cur = Some(p.clone());
        let mut hops = 0;
        while let Some(node) = cur {
            if node == id {
                return Err(StatusCode::CONFLICT);
            }
            if hops > 10_000 {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            cur = conn
                .query_row(
                    "SELECT parent_id FROM block WHERE id = ?1",
                    [&node],
                    |row| row.get::<_, Option<String>>(0),
                )
                .map_err(|_| StatusCode::NOT_FOUND)?;
            hops += 1;
        }
    }

    if let Some(ref p) = new_parent {
        let ok: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM block WHERE id = ?1 AND page_id = (SELECT page_id FROM block WHERE id = ?2)",
                [p, &id],
                |row| Ok(row.get::<_, i64>(0)? == 1),
            )
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !ok {
            return Err(StatusCode::CONFLICT);
        }
    }

    if let Some(ord) = body.get("ord").and_then(|o| o.as_f64()) {
        conn.execute(
            "UPDATE block SET parent_id = ?2, ord = ?3 WHERE id = ?1",
            rusqlite::params![id, new_parent, ord],
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        conn.execute(
            "UPDATE block SET parent_id = ?2 WHERE id = ?1",
            rusqlite::params![id, new_parent],
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(StatusCode::NO_CONTENT)
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
    if let Some(ord) = body.get("ord").and_then(|o| o.as_f64()) {
        conn.execute(
            "UPDATE block SET ord = ?2 WHERE id = ?1",
            rusqlite::params![id, ord],
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    if let Some(t) = body.get("type").and_then(|t| t.as_str()) {
        conn.execute(
            "UPDATE block SET type = ?2 WHERE id = ?1",
            rusqlite::params![id, t],
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

// ---- databases ----

async fn list_databases(State(db): State<Db>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.lock().await;
    let mut stmt = conn
        .prepare("SELECT id, name, properties FROM data_source ORDER BY created_at")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let dbs: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            let props: String = row.get(2)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "properties": serde_json::from_str::<serde_json::Value>(&props).unwrap_or(json!([])),
            }))
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!(dbs)))
}

async fn create_database(
    State(db): State<Db>,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let conn = db.lock().await;
    let id = uuid::Uuid::new_v4().to_string();
    let name = body.get("name").and_then(|n| n.as_str()).unwrap_or("Untitled database");
    let default_props = json!([{ "id": "name", "name": "Name", "type": "title" }]);
    conn.execute(
        "INSERT INTO data_source (id, workspace_id, name, properties) VALUES (?1, 'default', ?2, ?3)",
        rusqlite::params![id, name, default_props.to_string()],
    )
    .map_err(|e| { eprintln!("create_database error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn get_database(
    State(db): State<Db>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.lock().await;
    let mut stmt = conn
        .prepare("SELECT id, name, properties FROM data_source WHERE id = ?1")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rows = stmt
        .query_map([id], |row| {
            let props: String = row.get(2)?;
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "properties": serde_json::from_str::<serde_json::Value>(&props).unwrap_or(json!([])),
            }))
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match rows.next() {
        Some(Ok(dbrow)) => Ok(Json(dbrow)),
        _ => Err(StatusCode::NOT_FOUND),
    }
}

async fn delete_database(
    State(db): State<Db>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let conn = db.lock().await;
    conn.execute("DELETE FROM property_value WHERE data_source_id = ?1", [&id])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    conn.execute("DELETE FROM row_membership WHERE data_source_id = ?1", [&id])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    conn.execute("DELETE FROM data_source WHERE id = ?1", [&id])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Rows are pages: list all member pages with their property values.
async fn list_rows(
    State(db): State<Db>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.lock().await;
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.title, p.icon, pv.prop_id, pv.value
             FROM row_membership rm
             JOIN page p ON p.id = rm.page_id AND p.deleted = 0
             LEFT JOIN property_value pv ON pv.page_id = p.id AND pv.data_source_id = rm.data_source_id
             WHERE rm.data_source_id = ?1
             ORDER BY p.created_at",
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = stmt
        .query_map([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut out: Vec<serde_json::Value> = Vec::new();
    for (pid, title, icon, prop_id, value) in rows {
        let idx = out.iter().position(|r| r["id"] == pid);
        let row = match idx {
            Some(i) => &mut out[i],
            None => {
                out.push(json!({ "id": pid, "title": title, "icon": icon, "props": {} }));
                out.last_mut().unwrap()
            }
        };
        if let (Some(p), Some(v)) = (prop_id, value) {
            row["props"][p] = serde_json::from_str::<serde_json::Value>(&v).unwrap_or(json!(null));
        }
    }
    Ok(Json(json!(out)))
}

/// Create a row: a page that becomes a member of the database.
async fn create_row(
    State(db): State<Db>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let conn = db.lock().await;
    let pid = uuid::Uuid::new_v4().to_string();
    let title = body.get("title").and_then(|t| t.as_str()).unwrap_or("");
    conn.execute(
        "INSERT INTO page (id, workspace_id, title) VALUES (?1, 'default', ?2)",
        rusqlite::params![pid, title],
    )
    .map_err(|e| { eprintln!("create_row page error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
    conn.execute(
        "INSERT INTO row_membership (data_source_id, page_id) VALUES (?1, ?2)",
        rusqlite::params![id, pid],
    )
    .map_err(|e| { eprintln!("create_row membership error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok((StatusCode::CREATED, Json(json!({ "id": pid }))))
}

/// Add a property (column) to the database schema. Stable IDs.
async fn add_property(
    State(db): State<Db>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    let conn = db.lock().await;
    let (name, ptype) = (
        body.get("name").and_then(|n| n.as_str()).unwrap_or("Property"),
        body.get("type").and_then(|t| t.as_str()).unwrap_or("text"),
    );
    let current: String = conn
        .query_row("SELECT properties FROM data_source WHERE id = ?1", [&id], |r| r.get(0))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let mut props: Vec<serde_json::Value> = serde_json::from_str(&current).unwrap_or_default();
    let prop_id = format!("p{}", props.len() + 1);
    props.push(json!({ "id": prop_id, "name": name, "type": ptype }));
    conn.execute(
        "UPDATE data_source SET properties = ?2 WHERE id = ?1",
        rusqlite::params![id, serde_json::to_string(&props).unwrap()],
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Set one property value on one row.
async fn set_property(
    State(db): State<Db>,
    Path((id, page_id, prop_id)): Path<(String, String, String)>,
    Json(value): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    let conn = db.lock().await;
    conn.execute(
        "INSERT INTO property_value (data_source_id, page_id, prop_id, value) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (data_source_id, page_id, prop_id) DO UPDATE SET value = excluded.value",
        rusqlite::params![id, page_id, prop_id, value.to_string()],
    )
    .map_err(|e| { eprintln!("set_property error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(StatusCode::NO_CONTENT)
}
