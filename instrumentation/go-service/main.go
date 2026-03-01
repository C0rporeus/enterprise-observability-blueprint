package main

import (
	"context"
	"fmt"
	"log"
	"log/slog"
	"math/rand/v2"
	"net/http"
	"os"
	"os/signal"
	"time"

	"go.opentelemetry.io/contrib/bridges/otelslog"
	"go.opentelemetry.io/contrib/instrumentation/net/http/otelhttp"
	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/attribute"
	"go.opentelemetry.io/otel/propagation"
	"go.opentelemetry.io/otel/exporters/otlp/otlplog/otlploggrpc"
	"go.opentelemetry.io/otel/exporters/otlp/otlpmetric/otlpmetricgrpc"
	"go.opentelemetry.io/otel/exporters/otlp/otlptrace/otlptracegrpc"
	"go.opentelemetry.io/otel/metric"
	sdklog "go.opentelemetry.io/otel/sdk/log"
	sdkmetric "go.opentelemetry.io/otel/sdk/metric"
	"go.opentelemetry.io/otel/sdk/resource"
	sdktrace "go.opentelemetry.io/otel/sdk/trace"
	semconv "go.opentelemetry.io/otel/semconv/v1.21.0"
)

var (
	tracer = otel.Tracer("go-service")
	meter  = otel.Meter("go-service-meter")
)

func initProvider(ctx context.Context) (func(context.Context) error, *slog.Logger, error) {
	otelEndpoint := os.Getenv("OTEL_EXPORTER_OTLP_ENDPOINT")
	if otelEndpoint == "" {
		otelEndpoint = "otel-collector:4317"
	}

	serviceName := os.Getenv("OTEL_SERVICE_NAME")
	if serviceName == "" {
		serviceName = "go-service"
	}

	res, err := resource.New(ctx,
		resource.WithAttributes(
			semconv.ServiceName(serviceName),
			semconv.ServiceVersion("1.0.0"),
		),
	)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to create resource: %w", err)
	}

	// Set up Trace Provider
	traceExporter, err := otlptracegrpc.New(ctx, otlptracegrpc.WithInsecure(), otlptracegrpc.WithEndpoint(otelEndpoint))
	if err != nil {
		return nil, nil, fmt.Errorf("failed to create trace exporter: %w", err)
	}
	bsp := sdktrace.NewBatchSpanProcessor(traceExporter)
	tracerProvider := sdktrace.NewTracerProvider(
		sdktrace.WithSampler(sdktrace.AlwaysSample()),
		sdktrace.WithResource(res),
		sdktrace.WithSpanProcessor(bsp),
	)
	otel.SetTracerProvider(tracerProvider)

	// Set up W3C Trace Context propagator for cross-service correlation
	otel.SetTextMapPropagator(propagation.TraceContext{})

	// Set up Meter Provider
	metricExporter, err := otlpmetricgrpc.New(ctx, otlpmetricgrpc.WithInsecure(), otlpmetricgrpc.WithEndpoint(otelEndpoint))
	if err != nil {
		return nil, nil, fmt.Errorf("failed to create metric exporter: %w", err)
	}
	meterProvider := sdkmetric.NewMeterProvider(
		sdkmetric.WithResource(res),
		sdkmetric.WithReader(sdkmetric.NewPeriodicReader(metricExporter, sdkmetric.WithInterval(5*time.Second))),
	)
	otel.SetMeterProvider(meterProvider)

	// Set up Log Provider
	logExporter, err := otlploggrpc.New(ctx, otlploggrpc.WithInsecure(), otlploggrpc.WithEndpoint(otelEndpoint))
	if err != nil {
		return nil, nil, fmt.Errorf("failed to create log exporter: %w", err)
	}
	logProvider := sdklog.NewLoggerProvider(
		sdklog.WithResource(res),
		sdklog.WithProcessor(sdklog.NewBatchProcessor(logExporter)),
	)

	// Create slog logger backed by OTel
	logger := otelslog.NewLogger("go-service", otelslog.WithLoggerProvider(logProvider))

	return func(ctx context.Context) error {
		err1 := tracerProvider.Shutdown(ctx)
		err2 := meterProvider.Shutdown(ctx)
		err3 := logProvider.Shutdown(ctx)
		if err1 != nil {
			return err1
		}
		if err2 != nil {
			return err2
		}
		return err3
	}, logger, nil
}

func main() {
	ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt)
	defer cancel()

	shutdown, logger, err := initProvider(ctx)
	if err != nil {
		log.Fatalf("Failed to initialize OpenTelemetry: %v", err)
	}
	defer func() {
		if err := shutdown(context.Background()); err != nil {
			log.Printf("Failed to shutdown OpenTelemetry: %v", err)
		}
	}()

	// Define custom business metrics
	ordersCounter, _ := meter.Int64Counter("business.orders.created", metric.WithDescription("Number of orders created"))
	orderValue, _ := meter.Float64Counter("business.order.value", metric.WithDescription("Total value of orders"))

	// HTTP Handler with OTel instrumentation
	handler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()
		_, span := tracer.Start(ctx, "process_order")
		defer span.End()

		// Simulate business logic
		sleepTime := time.Duration(rand.IntN(200)+50) * time.Millisecond
		time.Sleep(sleepTime)

		// Create a random order
		price := rand.Float64()*100.0 + 10.0
		ordersCounter.Add(ctx, 1, metric.WithAttributes(attribute.String("status", "success")))
		orderValue.Add(ctx, price, metric.WithAttributes(attribute.String("currency", "USD")))

		span.SetAttributes(
			attribute.Float64("order.price", price),
			attribute.String("order.currency", "USD"),
		)

		// Random errors for RED/USE models
		if rand.IntN(100) < 5 {
			w.WriteHeader(http.StatusInternalServerError)
			w.Write([]byte("Internal Server Error\n"))
			span.RecordError(fmt.Errorf("simulated database failure"))
			logger.ErrorContext(ctx, "Database connection timeout", "order.price", price)
			return
		}

		logger.InfoContext(ctx, "Order processed", "order.price", price, "order.currency", "USD")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("Order processed\n"))
	})

	// Start a routine to generate constant fake traffic
	go func() {
		client := &http.Client{Transport: otelhttp.NewTransport(http.DefaultTransport)}
		for {
			select {
			case <-ctx.Done():
				return
			default:
				time.Sleep(1 * time.Second)
				client.Get("http://localhost:8080/order")
			}
		}
	}()

	wrappedHandler := otelhttp.NewHandler(handler, "http.server")
	http.Handle("/order", wrappedHandler)

	logger.Info("Go Service listening on :8080")
	if err := http.ListenAndServe(":8080", nil); err != nil {
		log.Fatal(err)
	}
}
