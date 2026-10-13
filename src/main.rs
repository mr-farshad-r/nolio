mod api;
mod db;
mod blocks;

fn main() {
    let runtime = tokio::runtime::Runtime::new().expect("failed to build tokio runtime");
    runtime.block_on(async {
        let db_path = std::env::var("NOLIO_DB").unwrap_or_else(|_| "nolio.db".into());
        let conn = db::new_db(db::init(&db_path).expect("failed to open database"));
        let port: u16 = std::env::var("NOLIO_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8787);
        let app = api::router(conn.clone());
        let addr = format!("0.0.0.0:{port}");
        println!("Nolio listening on http://{addr}");
        let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind failed");
        axum::serve(listener, app).await.expect("server error");
    });
}
