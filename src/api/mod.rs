use std::collections::HashMap;
use std::sync::Arc;
use actix_web::{delete, get, HttpResponse, post, Scope, web};
use actix_web::web::{Bytes, Data, Json, Query};
use serde::{Deserialize, Serialize};
use crate::repository::data_repository::DataRepository;

#[derive(Debug, Deserialize)]
pub struct ReadQuery {
    key: String
}

#[derive(Debug, Deserialize)]
pub struct WriteQuery {
    key: String,
    lifetime: Option<u32>
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    lifetime: Option<u32>
}

#[derive(Debug, Serialize)]
pub struct KeysReturn {
    keys: Vec<String>
}

#[get("/read")]
pub async fn read(query: Query<ReadQuery>, data: Data<Arc<DataRepository>>) -> HttpResponse {
    match DataRepository::read(data.as_ref().clone(), &query.key).await{
        None => { HttpResponse::NotFound().body("") }
        Some(v) => { HttpResponse::Ok()
            .content_type("application/octet-stream")
            .body(v) }
    }
}
#[get("/safe-read")]
pub async fn safe_read(query: Query<ReadQuery>, data: Data<Arc<DataRepository>>) -> HttpResponse {
    match data.safe_read(&query.key).await{
        None => { HttpResponse::NotFound().body("") }
        Some(v) => { HttpResponse::Ok()
            .content_type("application/octet-stream")
            .body(v) }
    }
}

#[get("/lifetime-read")]
pub async fn lifetime_read(query: Query<ReadQuery>, data: Data<Arc<DataRepository>>) -> HttpResponse {
    match data.lifetime_read(&query.key).await{
        None => { HttpResponse::NotFound().body("") }
        Some(v) => { HttpResponse::Ok()
            .content_type("application/octet-stream")
            .body(v.to_be_bytes().to_vec()) }
    }
}

#[delete("/remove")]
pub async fn remove(query: Query<ReadQuery>, data: Data<Arc<DataRepository>>) -> HttpResponse {
    data.remove(&query.key).await;
    HttpResponse::Ok()
        .body("")
}

fn handle_utf8(opt: Option<Vec<u8>>) -> HttpResponse {
    match opt {
        None => { HttpResponse::NotFound().body("") }
        Some(v) => {
            match String::from_utf8(v) {
                Ok(string) => HttpResponse::Ok()
                    .content_type("text/plain")
                    .body(string),
                Err(_) => HttpResponse::ImATeapot()
                    .body("")
            }
        }
    }
}

#[get("/read-string")]
pub async fn read_string(query: Query<ReadQuery>, data: Data<Arc<DataRepository>>) -> HttpResponse {
    handle_utf8(DataRepository::read(data.as_ref().clone(), &query.key).await)
}
#[get("/safe-read-string")]
pub async fn safe_read_string(query: Query<ReadQuery>, data: Data<Arc<DataRepository>>) -> HttpResponse {
    handle_utf8(data.safe_read(&query.key).await)
}

#[get("/all-keys")]
pub async fn all_keys(data: Data<Arc<DataRepository>>) -> Json<KeysReturn> {
    let keys = data.all_keys().await;
    Json(KeysReturn{
        keys
    })
}

#[get("/dump")]
pub async fn dump(data: Data<Arc<DataRepository>>) -> HttpResponse {
    let dump = data.dump().await;
    HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(dump)
}

#[get("/dump-json")]
pub async fn dump_json(data: Data<Arc<DataRepository>>) -> Json<HashMap<String, String>> {
    let all = data.dump_json().await;
    Json(all)
}

#[post("/load")]
pub async fn load(limit: Query<LimitQuery>, dump_data: Bytes, data: Data<Arc<DataRepository>>) -> HttpResponse {
    let limit = match limit.lifetime {
        Some(v) => if v == 0 { u16::MAX as u32 } else { v },
        None => u16::MAX as u32
    };
    let affected = DataRepository::load(data.as_ref().clone(), dump_data.to_vec(),
                                        limit).await;
    HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(affected.to_be_bytes().to_vec())
}

#[post("/load-json")]
pub async fn load_json(limit: Query<LimitQuery>, dump_data: Json<HashMap<String, String>>, data: Data<Arc<DataRepository>>) -> HttpResponse {
    let limit = match limit.lifetime {
        Some(v) => if v == 0 { u16::MAX as u32 } else { v },
        None => u16::MAX as u32
    };
    let affected = data.load_json(dump_data.clone(), limit).await;
    HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(affected.to_be_bytes().to_vec())
}

#[post("/write")]
pub async fn write(query: Query<WriteQuery>, body: Bytes, data: Data<Arc<DataRepository>>) -> HttpResponse {
    let limit = match query.lifetime {
        Some(v) => if v == 0 { u16::MAX as u32 } else { v },
        None => u16::MAX as u32
    };
    data.write(query.key.clone(), body.to_vec(), limit).await;
    HttpResponse::Ok().body("")
}

#[post("/write-string")]
pub async fn write_string(query: Query<WriteQuery>, body: String, data: Data<Arc<DataRepository>>) -> HttpResponse {
    let limit = match query.lifetime {
        Some(v) => if v == 0 { u16::MAX as u32 } else { v },
        None => u16::MAX as u32
    };
    data.write_string(query.key.clone(), body, limit).await;
    HttpResponse::Ok()
        .body("")
}

pub fn map() -> Scope {
    web::scope("")
        .service(read)
        .service(safe_read)
        .service(read_string)
        .service(safe_read_string)
        .service(lifetime_read)
        .service(all_keys)
        .service(dump_json)
        .service(dump)
        .service(load_json)
        .service(load)
        .service(write)
        .service(write_string)
}