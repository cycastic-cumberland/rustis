use std::sync::Arc;
use actix_web::{App, HttpServer};
use actix_web::middleware::Logger;
use actix_web::web::Data;
use log::{Level, log};
use tokio::fs;
use crate::api::map;
use crate::config::ApplicationConfig;
use crate::repository::data_repository::DataRepository;

mod repository;
mod config;
mod api;

const HOST: &str = "0.0.0.0";
const PORT: u16 = 8288;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cfg_path = match std::env::var("APP_CONFIG_PATH"){
        Ok(v) => v,
        Err(_) => panic!("Environment variable APP_CONFIG_PATH not set")
    };
    let cfg_content = match fs::read_to_string(cfg_path).await {
        Ok(v) => v,
        Err(e) => panic!("Failed to read application configuration with exception: {}", e.to_string())
    };
    let config: ApplicationConfig = match serde_json::from_str(&cfg_content) {
        Ok(cfg) => cfg,
        Err(e) => panic!("Failed to deserialize application configuration with exception: {}", e.to_string())
    };
    let partition_count = config.partition_count;
    let repository = Arc::new(DataRepository::new(partition_count));
    std::env::set_var("RUST_LOG", config.log_level);
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    log!(Level::Info, "Partition count: {partition_count}");
    log!(Level::Info, "Online at {HOST}:{PORT}");

    HttpServer::new(move || {
        let logger = Logger::default();
        let cloned_repo = repository.clone();
        App::new()
            .wrap(logger)
            .app_data(Data::new(cloned_repo))
            .service(map())

    })
        .bind((HOST, PORT))?
        .run()
        .await
}
