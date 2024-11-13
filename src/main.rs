use axum::extract::{Multipart, Path};
use axum::http::header;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Extension, Router};

use sqlx::Row;
use tokio_util::io::ReaderStream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;

    let db_url = std::env::var("DATABASE_URL")?;
    let db_pool = sqlx::SqlitePool::connect(&db_url).await?;
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    let app = Router::new()
        .route("/", get(home))
        .route("/:id", get(get_image))
        .route("/upload", post(upload))
        .layer(Extension(db_pool));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn home() -> Html<String> {
    let path = std::path::Path::new("src/index.html");
    let contents = tokio::fs::read_to_string(path).await.unwrap();
    Html(contents)
}

async fn insert_image(db_pool: &sqlx::SqlitePool, tags: &str) -> anyhow::Result<i64> {
    let row = sqlx::query("INSERT INTO images (tags) VALUES (?) RETURNING id")
        .bind(tags)
        .fetch_one(db_pool)
        .await?;

    Ok(row.get(0))
}

async fn save_image(id: i64, bytes: &[u8]) -> anyhow::Result<()> {
    let base_path = std::path::Path::new("images");
    if !base_path.exists() || !base_path.is_dir() {
        tokio::fs::create_dir_all(base_path).await?;
    }

    let image_path = base_path.join(format!("{id}.jpg"));
    if image_path.exists() {
        anyhow::bail!("File already exists");
    }

    tokio::fs::write(image_path, bytes).await?;
    Ok(())
}

async fn upload(
    Extension(db_pool): Extension<sqlx::SqlitePool>,
    mut multipart: Multipart,
) -> String {
    let mut tags = None;
    let mut img = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        match name.as_str() {
            "tags" => tags = Some(String::from_utf8(data.to_vec()).unwrap()),
            "image" => img = Some(data.to_vec()),
            _ => panic!("Unknown field {name}"),
        }
    }

    if let (Some(tags), Some(img)) = (tags, img) {
        let new_image_id = insert_image(&db_pool, &tags).await.unwrap();
        save_image(new_image_id, &img).await.unwrap();
    } else {
        panic!("Missing fields")
    }
    "Ok".to_string()
}

async fn get_image(Path(id): Path<i64>) -> impl IntoResponse {
    let filename = format!("images/{id}.jpg");
    let attachment = format!("filename={filename}");

    let file = tokio::fs::File::open(&filename).await.unwrap();

    axum::response::Response::builder()
        .header(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("image/jpeg"),
        )
        .header(
            header::CONTENT_DISPOSITION,
            header::HeaderValue::from_str(&attachment).unwrap(),
        )
        .body(axum::body::Body::from_stream(ReaderStream::new(file)))
        .unwrap()
}
