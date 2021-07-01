use actix_web::{get, http::header, post, web, App, HttpResponse, HttpServer, ResponseError};
// テンプレート
use askama::Template;
// データベース
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
// POSTデータパース
use serde::Deserialize;
// エラー
use thiserror::Error;

// ==================================================
// Params
// ==================================================

#[derive(Deserialize)]
struct AddParams {
	text: String,
}

#[derive(Deserialize)]
struct DeleteParams {
	id: u32,
}

// ==================================================
// Value of template
// ==================================================

#[allow(dead_code)]
struct TodoEntry {
	id: u32,
	text: String,
}

// ==================================================
// Templates
// ==================================================

#[derive(Template)]
// パスを指定
#[template(path = "index.html")]
struct IndexTemplate {
	entries: Vec<TodoEntry>,
}

// ==================================================
// Error
// ==================================================

#[derive(Error, Debug)]
enum MyError {
	#[error("Faild to render HTML")]
	AskamaError(#[from] askama::Error),

	#[error("Faild to get connection")]
	ConnectionPoolError(#[from] r2d2::Error),

	#[error("Faild SQL execution")]
	SQLiteError(#[from] rusqlite::Error),
}

impl ResponseError for MyError {}

// MyErrorはactix_web::ResponseErrorを実装しているので、indexの戻り値にMyErrorを使うことができる

// ==================================================
// Response
// ==================================================

#[post("/add")]
async fn add_todo(
	params: web::Form<AddParams>,
	db: web::Data<r2d2::Pool<SqliteConnectionManager>>,
) -> Result<HttpResponse, MyError> {
	let conn = db.get()?;
	conn.execute("INSERT INTO todo (text) VALUES (?)", &[&params.text])?;
	Ok(HttpResponse::SeeOther()
		.header(header::LOCATION, "/")
		.finish())
}

#[post("/delete")]
async fn delete_todo(
	params: web::Form<DeleteParams>,
	db: web::Data<r2d2::Pool<SqliteConnectionManager>>,
) -> Result<HttpResponse, MyError> {
	let conn = db.get()?;
	conn.execute("DELETE FROM todo WHERE id=?", &[&params.id])?;
	Ok(HttpResponse::SeeOther()
		.header(header::LOCATION, "/")
		.finish())
}

/// When receiving a GET request with the path "/".
#[get("/")]
async fn index(db: web::Data<Pool<SqliteConnectionManager>>) -> Result<HttpResponse, MyError> {
	// データベースからデータを取得
	let conn = db.get()?;
	let mut statement = conn.prepare("SELECT id, text FROM todo")?;
	let rows = statement.query_map(params![], |row| {
		let id = row.get(0)?;
		let text = row.get(1)?;
		Ok(TodoEntry {id, text})
	})?;

	// 表示内容
	let mut entries = Vec::new();
	// entries.push(TodoEntry {
	//     id: 1,
	//     text: "First entry".to_string(),
	// });
	// entries.push(TodoEntry {
	//     id: 2,
	//     text: "Second entry".to_string(),
	// });
	for row in rows {
		entries.push(row?);
	}
	let html = IndexTemplate { entries };
	// レスポンスの内容を指定
	let response_body = html.render()?;
	// レスポンスを返す
	Ok(HttpResponse::Ok()
		.content_type("text/html")
		.body(response_body))
}

// ==================================================
// Server
// ==================================================

/// Server setup and startup.
#[actix_rt::main]
async fn main() -> Result<(), actix_web::Error> {
	let manager = SqliteConnectionManager::file("todo.db");
	let pool = Pool::new(manager).expect("Failed to initialize the connection pool");
	let conn = pool
		.get()
		.expect("Failed to get the connection from the pool.");
	conn.execute(
		"CREATE TABLE IF NOT EXISTS todo (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			text TEXT NOT NULL
		)",
		params![],
	)
	.expect("Failed to create a table `todo`.");

	// 新しいサーバーオブジェクトを生成
	HttpServer::new(move || {
		App::new()
			.service(index)
			.service(add_todo)
			.service(delete_todo)
			.data(pool.clone())
	})
		// URLを指定
		.bind("0.0.0.0:8080")?
		// サーバーを起動
		.run()
		// 非同期通信
		.await?;
	Ok(())
}