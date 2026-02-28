use axum::{Router, http::StatusCode, routing::get};
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{error, info, instrument};
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;

// Inicializa el pipeline de OpenTelemetry (Trazas)
fn init_tracer() -> SdkTracerProvider {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://otel-collector:4317".to_string());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("Failed to create OTLP span exporter");

    let resource = Resource::builder_empty()
        .with_attributes([
            KeyValue::new("service.name", "rust-service"),
            KeyValue::new("service.version", "1.0.0"),
        ])
        .build();

    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    let _ = global::set_tracer_provider(tracer_provider.clone());
    tracer_provider
}

#[instrument]
async fn process_order() -> (StatusCode, &'static str) {
    // Simulate work
    let sleep_time = rand::random_range(50..200);
    tokio::time::sleep(Duration::from_millis(sleep_time)).await;

    let price: f64 = rand::random_range(10.0..110.0);

    // Add custom event to trace
    tracing::info!(
        order.price = price,
        order.currency = "EUR",
        "Order processed in rust-service"
    );

    // Simulate random errors for RED model
    if rand::random_range(0..100) < 5 {
        error!("Database connection timeout");
        return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error");
    }

    (StatusCode::OK, "Order processed by Rust")
}

#[tokio::main]
async fn main() {
    // Setup Tracer
    let tracer_provider = init_tracer();
    let tracer = global::tracer("rust-service");
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let app = Router::new().route("/order", get(process_order));

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

    let _ = tracer_provider.shutdown();
}
