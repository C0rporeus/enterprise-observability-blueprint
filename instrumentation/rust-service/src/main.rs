use axum::{Router, http::StatusCode, routing::get};
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{error, info, instrument};
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;

fn otel_resource() -> Resource {
    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "rust-service".to_string());

    Resource::builder_empty()
        .with_attributes([
            KeyValue::new("service.name", service_name),
            KeyValue::new("service.version", "1.0.0"),
        ])
        .build()
}

fn otel_endpoint() -> String {
    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://otel-collector:4317".to_string())
}

fn init_tracer(resource: Resource, endpoint: &str) -> SdkTracerProvider {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("Failed to create OTLP span exporter");

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    let _ = global::set_tracer_provider(provider.clone());
    provider
}

fn init_meter(resource: Resource, endpoint: &str) -> SdkMeterProvider {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("Failed to create OTLP metric exporter");

    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(5))
        .build();

    let provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build();

    global::set_meter_provider(provider.clone());
    provider
}

fn init_logger(resource: Resource, endpoint: &str) -> SdkLoggerProvider {
    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("Failed to create OTLP log exporter");

    SdkLoggerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build()
}

#[instrument]
async fn process_order() -> (StatusCode, &'static str) {
    let meter = global::meter("rust-service-meter");
    let orders_counter = meter
        .u64_counter("business.orders.created")
        .with_description("Number of orders created")
        .build();
    let order_value_counter = meter
        .f64_counter("business.order.value")
        .with_description("Total value of orders")
        .build();

    // Simulate work
    let sleep_time = rand::random_range(50..200);
    tokio::time::sleep(Duration::from_millis(sleep_time)).await;

    let price: f64 = rand::random_range(10.0..110.0);

    // Record business metrics
    orders_counter.add(1, &[KeyValue::new("status", "success")]);
    order_value_counter.add(price, &[KeyValue::new("currency", "EUR")]);

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
    let resource = otel_resource();
    let endpoint = otel_endpoint();

    // Initialize all three signal providers
    let tracer_provider = init_tracer(resource.clone(), &endpoint);
    let meter_provider = init_meter(resource.clone(), &endpoint);
    let logger_provider = init_logger(resource, &endpoint);

    // Build tracing subscriber with OTel trace + log layers
    let tracer = global::tracer("rust-service");
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let log_layer =
        opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&logger_provider);

    let subscriber = Registry::default().with(telemetry_layer).with(log_layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let app = Router::new().route("/order", get(process_order));

    // Spawn traffic generator with trace context propagation
    tokio::spawn(async {
        // Wait for the server to start
        tokio::time::sleep(Duration::from_secs(2)).await;
        let client = reqwest::Client::new();
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let tracer = global::tracer("rust-service-traffic");
            use opentelemetry::trace::Tracer;
            let _span = tracer.start("traffic-generator");
            let _ = client.get("http://127.0.0.1:8081/order").send().await;
        }
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
    info!("Rust Service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    // Graceful shutdown of all providers
    let _ = tracer_provider.shutdown();
    let _ = meter_provider.shutdown();
    let _ = logger_provider.shutdown();
}
