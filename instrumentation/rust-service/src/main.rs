use axum::{
    routing::get,
    Router,
    response::IntoResponse,
    http::StatusCode,
};
use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::{
    trace::{self, Tracer},
    Resource,
};
use opentelemetry_semantic_conventions::resource;
use rand::Rng;
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{error, info, instrument};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

// Inicializa el pipeline de OpenTelemetry (Trazas)
fn init_tracer() -> Tracer {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://otel-collector:4317".to_string());

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint);

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            trace::config().with_resource(Resource::new(vec![
                KeyValue::new(resource::SERVICE_NAME, "rust-service"),
                KeyValue::new(resource::SERVICE_VERSION, "1.0.0"),
            ])),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("Failed to initialize OpenTelemetry tracer")
}

#[instrument]
async fn process_order() -> impl IntoResponse {
    let mut rng = rand::thread_rng();
    
    // Simulate work
    let sleep_time = rng.gen_range(50..200);
    tokio::time::sleep(Duration::from_millis(sleep_time)).await;

    let price: f64 = rng.gen_range(10.0..110.0);
    
    // Add custom event to trace
    tracing::info!(
        order.price = price,
        order.currency = "EUR",
        "Order processed in rust-service"
    );

    // Simulate random errors for RED model
    if rng.gen_range(0..100) < 5 {
        error!("Database connection timeout");
        return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error");
    }

    (StatusCode::OK, "Order processed by Rust")
}

#[tokio::main]
async fn main() {
    // Setup Tracer
    let tracer = init_tracer();
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let app = Router::new()
        .route("/order", get(process_order));

    // Spawn traffic generator
    tokio::spawn(async {
        let client = reqwest::Client::new();
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = client.get("http://127.0.0.1:8081/order").send().await;
        }
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
    info!("Rust Service listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    global::shutdown_tracer_provider();
}
