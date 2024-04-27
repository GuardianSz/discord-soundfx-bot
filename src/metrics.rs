use axum::{routing::get, Router};
use lazy_static;
use log::warn;
use prometheus::{register_int_counter, IntCounter, Registry};

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    pub static ref PLAY_COUNTER: IntCounter =
        register_int_counter!("play_cmd", "Number of calls to /play").unwrap();
    pub static ref UPLOAD_COUNTER: IntCounter =
        register_int_counter!("upload_cmd", "Number of calls to /upload").unwrap();
    pub static ref DELETE_COUNTER: IntCounter =
        register_int_counter!("delete_cmd", "Number of calls to /delete").unwrap();
    pub static ref GREET_COUNTER: IntCounter =
        register_int_counter!("greet_invoke", "Number of greet sounds played").unwrap();
}

pub fn init_metrics() {
    REGISTRY.register(Box::new(PLAY_COUNTER.clone())).unwrap();
    REGISTRY.register(Box::new(UPLOAD_COUNTER.clone())).unwrap();
    REGISTRY.register(Box::new(DELETE_COUNTER.clone())).unwrap();
    REGISTRY.register(Box::new(GREET_COUNTER.clone())).unwrap();
}

pub async fn serve() {
    let app = Router::new().route("/metrics", get(metrics));

    let listener = tokio::net::TcpListener::bind("localhost:31755")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn metrics() -> String {
    let encoder = prometheus::TextEncoder::new();
    let res_custom = encoder.encode_to_string(&REGISTRY.gather());

    match res_custom {
        Ok(s) => s,
        Err(e) => {
            warn!("Error encoding metrics: {:?}", e);

            String::new()
        }
    }
}
