use axum::body::{to_bytes, Body};
use axum::http::Request;
use axum::Router;
use sea_orm::Database;
use sea_orm_migration::MigratorTrait;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn request(app: &Router, method: &str, path: &str, ctype: &str, body: String) -> Value {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header("Content-Type", ctype)
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    assert!(
        status.is_success(),
        "{method} {path}: {status} {}",
        String::from_utf8_lossy(&bytes)
    );
    serde_json::from_slice(&bytes).unwrap()
}

async fn post(app: &Router, path: &str, body: Value) -> Value {
    request(app, "POST", path, "application/json", body.to_string()).await
}

#[tokio::test]
async fn import_edits_the_requested_row_and_ignores_tags_on_skipped_duplicates() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    migration::Migrator::up(&db, None).await.unwrap();
    let mut config = server::config::Config::default();
    config.dict.enabled = false;
    config.dict.plugins_poll_secs = 0;
    let app = server::router::build(db, None, config).unwrap();
    let book = post(&app, "/api/wordbooks", json!({"name": "导入测试"})).await;
    let book_url = format!("/api/wordbooks/{}", book["id"]);
    let words = format!("{book_url}/words");
    post(
        &app,
        &words,
        json!({"spelling": "apple", "definitions": [ {"pos":"n.","meaning":"原苹果"} ]}),
    )
    .await;

    let body = concat!("--upload\r\nContent-Disposition: form-data; name=\"file\"; filename=\"words.csv\"\r\n",
        "Content-Type: text/csv\r\n\r\n单词,音标,词性,释义,例句,标签\r\napple,,n.,苹果,,跳过的标签\r\n\r\n",
        "banana,,n.,待修正,,\r\ncherry,,n.,樱桃,,保留的标签\r\n--upload--\r\n");
    let preview = request(
        &app,
        "POST",
        &format!("{words}/import/preview"),
        "multipart/form-data; boundary=upload",
        body.into(),
    )
    .await;
    let changed = post(&app, &format!("{words}/import/rows"), json!({
        "token": preview["token"], "page": 1, "page_size": 20,
        "updates": [{"row": 4, "spelling": "banana", "phonetic": "", "pos": "n.", "meaning": "香蕉", "example": "", "tags": ""}]
    })).await;
    let rows = changed["rows"].as_array().unwrap();
    assert!(rows.iter().any(|r| r["row"] == 4 && r["meaning"] == "香蕉"));
    assert!(rows
        .iter()
        .any(|r| r["row"] == 5 && r["spelling"] == "cherry"));

    let imported = post(
        &app,
        &format!("{words}/import"),
        json!({"token": preview["token"], "update_rows": []}),
    )
    .await;
    assert_eq!(
        imported,
        json!({"imported": 2, "updated": 0, "skipped_errors": 0, "skipped_duplicates": 1, "created_tags": 1})
    );
    let tags = request(
        &app,
        "GET",
        &format!("{book_url}/tags"),
        "application/json",
        String::new(),
    )
    .await;
    assert_eq!(tags.as_array().unwrap().len(), 1);
    assert_eq!(tags[0]["name"], "保留的标签");
}
