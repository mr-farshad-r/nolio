use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type Db = Arc<Mutex<Connection>>;

pub fn new_db(conn: Connection) -> Db {
    Arc::new(Mutex::new(conn))
}

pub fn init(path: &str) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS workspace (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS page (
            id          TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL REFERENCES workspace(id),
            parent_id   TEXT REFERENCES page(id),
            title       TEXT NOT NULL DEFAULT '',
            icon        TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
            deleted     INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_page_parent ON page(parent_id);
        CREATE INDEX IF NOT EXISTS idx_page_workspace ON page(workspace_id);

        CREATE TABLE IF NOT EXISTS block (
            id          TEXT PRIMARY KEY,
            page_id     TEXT NOT NULL REFERENCES page(id),
            parent_id   TEXT REFERENCES block(id),
            type        TEXT NOT NULL,
            content     TEXT NOT NULL DEFAULT '{}',
            ord         REAL NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            deleted     INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_block_page ON block(page_id);
        CREATE INDEX IF NOT EXISTS idx_block_parent ON block(parent_id);
        "#,
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO workspace (id, name) VALUES ('default', 'Personal')",
        [],
    )?;
    Ok(conn)
}
