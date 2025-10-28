use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use clap::Parser;
use gmq_proxy::service::topic_config::{TopicConfig, TopicConfigManager};
use log::error;
use log::info;
use serde::{Deserialize, Serialize};
use simplelog::{CombinedLogger, TermLogger};
use std::{fs::File, io::Read, path::PathBuf, sync::Arc};

#[tokio::main]
async fn main() {
    CombinedLogger::init(vec![TermLogger::new(
        log::LevelFilter::Debug,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )])
    .unwrap();
    info!("start application...");
    let args = Arg::parse();
    let config_file = args.config;
    if !config_file.exists() {
        error!("config file {} is not present.", config_file.display());
        return;
    }
    let mut file = File::open(config_file).unwrap();
    let mut data = vec![];
    file.read_to_end(&mut data).unwrap();
    let config: Config = toml::from_slice(&data).unwrap();
    let topic_config_manager = Arc::new(TopicConfigManager::new(&config.config_path).unwrap());
    let admin_app = Router::new()
        .route("/create-topic", post(create_topic))
        .with_state(topic_config_manager);

    let admin_listener = tokio::net::TcpListener::bind("0.0.0.0:9527").await.unwrap();
    axum::serve(admin_listener, admin_app).await.unwrap();
}

#[derive(Parser, Default)]
struct Arg {
    #[arg(short, long, value_name = "FILE")]
    config: PathBuf,
}

#[derive(Deserialize)]
struct Config {
    config_path: String,
}

async fn create_topic(
    State(state): State<Arc<TopicConfigManager>>,
    Json(request): Json<CreateTopic>,
) -> (StatusCode, Json<BaseResponse<()>>) {
    let topic_config = TopicConfig::new(
        request.topic_name,
        request.queue_num,
        gmq_proxy::service::topic_config::TopicType::NORMAL,
    );
    let result = state.add_or_update_topic(topic_config);
    if result.is_err() {
        let error = BaseResponse {
            code: 1,
            message: result.unwrap_err().to_string(),
            data: None,
        };
        return (StatusCode::OK, Json(error));
    }

    let response = BaseResponse {
        code: 0,
        message: "ok".to_string(),
        data: None,
    };
    (StatusCode::OK, Json(response))
}

#[derive(Debug, Deserialize)]
pub struct CreateTopic {
    pub topic_name: String,
    queue_num: u32,
}

#[derive(Debug, Serialize)]
pub struct BaseResponse<T> {
    code: i32,
    message: String,
    data: Option<T>,
}
