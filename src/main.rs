use axum::{routing::get, Extension, Router};

use sqlx::Row;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;

    let db_url = std::env::var("DATABASE_URL")?;
    let db_pool = sqlx::SqlitePool::connect(&db_url).await?;
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    let app = Router::new()
        .route("/", get(test))
        .layer(Extension(db_pool));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn test(Extension(db_bool): Extension<sqlx::SqlitePool>) -> String {
    let result = sqlx::query("SELECT COUNT(id) FROM images")
        .fetch_one(&db_bool)
        .await
        .unwrap();

    let count = result.get::<i64, _>(0);
    format!("Count {count}")
}
