# Enterprise Observability Blueprint - Full Audit

**Date:** 2026-02-28
**Scope:** Structure, coherence, design patterns, architecture, potential improvements

---

## 1. Repository Structure

```
enterprise-observability-blueprint/
├── .github/workflows/ci.yml
├── Makefile
├── README.md
├── deploy/docker-compose/
│   ├── docker-compose.yaml
│   ├── prometheus.yml
│   ├── loki.yaml
│   └── tempo.yaml
├── docs/adrs/
│   ├── 0001-use-opentelemetry-as-standard.md
│   └── 0002-red-use-models-for-dashboards.md
├── grafana/
│   ├── dashboards/
│   │   ├── red-use-model.json
│   │   └── business-metrics.json
│   └── provisioning/
│       ├── dashboards/dashboards.yaml
│       └── datasources/datasources.yaml
├── instrumentation/
│   ├── go-service/
│   │   ├── Dockerfile
│   │   ├── go.mod / go.sum
│   │   └── main.go
│   └── rust-service/
│       ├── Dockerfile
│       ├── Cargo.toml / Cargo.lock
│       └── src/main.rs
└── otelcol/
    └── config.yaml
```

**Veredicto:** Estructura limpia y bien organizada. La separacion por concerns (infra, instrumentacion, visualizacion, docs) es correcta.

---

## 2. Scorecard

| Dimension                  | Score | Max |
|----------------------------|-------|-----|
| Structural Coherence       | 3.5   | 5   |
| OTel Pipeline Design       | 2.5   | 5   |
| Instrumentation Parity     | 2.0   | 5   |
| Cross-Signal Correlation   | 3.5   | 5   |
| Dashboard Quality          | 3.0   | 5   |
| Security Posture           | 2.0   | 5   |
| CI/CD                      | 3.0   | 5   |
| Documentation              | 3.5   | 5   |
| Operability (Docker)       | 2.5   | 5   |
| **TOTAL**                  | **25.5** | **45** |
| **Weighted Average**       | **2.83 / 5** | |

---

## 3. Flow de Uso (Flujo de Datos)

```
┌────────────┐  ┌──────────────┐
│ Go Service │  │ Rust Service  │
│ :8080      │  │ :8081         │
│ traces+    │  │ traces only   │  <-- ASIMETRIA
│ metrics    │  │               │
└─────┬──────┘  └──────┬────────┘
      │ OTLP gRPC      │ OTLP gRPC
      └────────┬────────┘
               ▼
     ┌──────────────────┐
     │  OTel Collector   │
     │  :4317 (gRPC)     │
     │  :4318 (HTTP)     │
     │  :8888 (self)     │
     └──┬────┬────┬──────┘
        │    │    │
        ▼    ▼    ▼
    Prom   Loki  Tempo
    :9090  :3100 :3200
        │    │    │
        └────┼────┘
             ▼
         Grafana :3000
         (datasources con
          cross-signal linking)
```

**Problema clave:** El Rust service solo emite trazas. No emite metricas ni logs. El pipeline de logs del Collector no tiene productor real.

---

## 4. Hallazgos

### CRITICO - Procesadores Definidos pero Nunca Usados

**Archivo:** `otelcol/config.yaml:29-42`

```yaml
# Definidos en processors:
filter/business:    # Incluye metricas que empiezan con 'business.'
filter/technical:   # Excluye metricas que empiezan con 'business.'
```

```yaml
# Pero el pipeline de metricas usa:
metrics:
  processors: [memory_limiter, resource/env, batch]  # <-- NO usa filter/*
```

El ADR-0001 dice: *"Filtrar y separar Metricas de Negocio de Metricas Tecnicas, enviandolas a sistemas de almacenamiento o retenciones diferentes"*. Esta capacidad esta declarada en la config pero **no esta conectada**. Los filtros son dead code.

**Impacto:** La propuesta central del proyecto (separacion business vs technical metrics) no funciona. Todas las metricas van al mismo pipeline sin separacion.

### CRITICO - Asimetria de Instrumentacion entre Servicios

| Signal   | Go Service        | Rust Service       |
|----------|-------------------|--------------------|
| Traces   | SDK + otelhttp    | SDK + #[instrument]|
| Metrics  | business.orders.created, business.order.value | NINGUNA |
| Logs     | No exporta a OTel | No exporta a OTel  |

**Impacto:**
- El dashboard `business-metrics.json` consulta `business_orders_created_total` y `business_order_value_total` que **solo el Go service produce**. El Rust service es invisible en KPIs de negocio.
- El pipeline de logs (`receivers: [otlp]` -> `exporters: [loki]`) **no recibe datos de ningun servicio**. El panel "Service Logs" del dashboard RED/USE estara vacio.
- El README promete "instrumentacion multi-lenguaje" pero la paridad no existe.

### ALTO - Grafana con Acceso Admin Anonimo

**Archivo:** `deploy/docker-compose/docker-compose.yaml:69-71`

```yaml
GF_AUTH_ANONYMOUS_ENABLED=true
GF_AUTH_ANONYMOUS_ORG_ROLE=Admin    # <-- Admin sin autenticacion
GF_AUTH_DISABLE_LOGIN_FORM=true
```

Aunque es un entorno de demo, dar rol `Admin` a usuarios anonimos permite:
- Modificar/eliminar datasources
- Borrar dashboards provisionados
- Cambiar configuracion de la org
- Crear API keys

**Recomendacion:** Usar `GF_AUTH_ANONYMOUS_ORG_ROLE=Viewer` como minimo.

### ALTO - Rust Service: Traffic Generator sin Context Propagation

**Archivo:** `instrumentation/rust-service/src/main.rs:74-80`

```rust
tokio::spawn(async {
    let client = reqwest::Client::new();  // <-- cliente raw sin instrumentacion
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let _ = client.get("http://127.0.0.1:8081/order").send().await;
    }
});
```

El cliente HTTP no propaga contexto de traza. Cada request auto-generado crea un span aislado sin relacion padre-hijo. Contraste con Go que usa `otelhttp.NewTransport`:

```go
client := &http.Client{Transport: otelhttp.NewTransport(http.DefaultTransport)}
```

### MEDIO - Loki Schema v11 Obsoleto

**Archivo:** `deploy/docker-compose/loki.yaml:19-25`

```yaml
schema_config:
  configs:
    - from: 2020-10-24
      store: boltdb-shipper    # <-- legacy
      object_store: filesystem
      schema: v11              # <-- obsoleto, actual es v13
```

Loki 2.9.x soporta schema v13 con TSDB store que ofrece mejor rendimiento en queries. `boltdb-shipper` esta en modo mantenimiento.

### MEDIO - Docker Compose: version Key Deprecated

**Archivo:** `deploy/docker-compose/docker-compose.yaml:1`

```yaml
version: '3.8'  # <-- deprecado en Compose Spec
```

Desde Docker Compose v2+, el campo `version` es ignorado y genera warnings. Debe eliminarse.

### MEDIO - Sin Health Checks ni Dependency Ordering Robusto

```yaml
depends_on:
  - prometheus
  - loki
  - tempo
```

`depends_on` sin `condition: service_healthy` solo garantiza orden de inicio, no que el servicio este ready. El OTel Collector puede intentar enviar a Prometheus/Loki/Tempo antes de que esten aceptando conexiones.

**Recomendacion:** Agregar `healthcheck` a cada backend y usar `depends_on` con condicion:

```yaml
depends_on:
  prometheus:
    condition: service_healthy
```

### MEDIO - Prometheus Config Incompleta

**Archivo:** `deploy/docker-compose/prometheus.yml`

```yaml
scrape_configs:
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']
```

Prometheus solo se scrapea a si mismo. No scrapea al OTel Collector (`:8888`) ni a los servicios directamente. Aunque el flujo es via OTLP remote-write, las metricas internas del Collector (queue sizes, export errors, retry counts) son clave para observar la salud del pipeline y no se estan recolectando.

### MEDIO - Dashboard RED/USE: Queries Dependen de Metricas de Span-Metrics Generator

Las queries del dashboard RED/USE usan:
- `http_server_duration_count` / `http_server_duration_bucket`
- `process_resident_memory_bytes`
- `process_cpu_seconds_total`

Las metricas `http_server_duration_*` provienen del **span-metrics generator de Tempo** (configurado en `tempo.yaml:21`), no del SDK de los servicios. Esto crea una dependencia implicita: si Tempo no genera span-metrics, el dashboard RED se queda vacio. Esta dependencia no esta documentada.

### BAJO - Sin .gitignore

No existe `.gitignore` para excluir:
- Volumenes Docker temporales
- Archivos `.env`
- Directorios `target/` (Rust) o binarios Go compilados localmente
- Archivos de IDE

### BAJO - Dockerfiles Usan `alpine:latest` como Runtime

**Archivos:** `instrumentation/*/Dockerfile`

```dockerfile
FROM alpine:latest   # <-- no pinned
```

Usar `latest` rompe la reproducibilidad. Si Alpine publica una version con cambios breaking, los builds fallan sin cambios en el codigo.

### BAJO - Go Service Usa `rand.Intn` Deprecado

**Archivo:** `instrumentation/go-service/main.go:105,109,119`

```go
sleepTime := time.Duration(rand.Intn(200)+50) * time.Millisecond
price := rand.Float64()*100 + 10
if rand.Intn(100) < 5 {
```

Go 1.22+ introdujo `rand.N[T]()` y `rand.IntN()` como reemplazo. El `Dockerfile` usa `golang:1.24-alpine`.

### OBSERVACION - SLOs/Error Budgets Prometidos pero No Implementados

El README dice: *"SLOs y Error Budgets: Visualizacion del nivel de servicio real percibido por el usuario"*. No hay:
- Alerting rules en Grafana
- Recording rules en Prometheus
- SLO dashboards o burn-rate panels

### OBSERVACION - Falta OTEL_SERVICE_NAME como Env Var

Ambos servicios hardcodean `service.name` en el SDK:

```go
semconv.ServiceName("go-service")
```

```rust
KeyValue::new("service.name", "rust-service")
```

La convencion OTel es usar `OTEL_SERVICE_NAME` como variable de entorno, lo que permite reutilizar el mismo binario con diferentes nombres de servicio sin recompilar.

### OBSERVACION - CI No Valida Dashboards JSON

El CI valida la config del OTel Collector y compila los servicios, pero no valida:
- Sintaxis JSON de los dashboards de Grafana
- Que los UID de datasource en los dashboards coincidan con los provisionados
- Que las queries PromQL sean sintacticamente correctas

---

## 5. Analisis de Coherencia Estructural

### Lo que funciona bien

1. **Separacion de concerns clara**: infra (deploy/), instrumentacion (instrumentation/), visualizacion (grafana/), docs (docs/adrs/)
2. **ADRs bien escritos**: Documentan el "por que" de las decisiones (OTel como estandar, RED/USE para dashboards)
3. **Cross-signal linking en Grafana**: Loki -> Tempo via `trace_id` derived field, Tempo -> Loki via `tracesToLogsV2`. El graph de nodos esta habilitado.
4. **OTel Collector como unico punto de ingesta**: Correcto per ADR-0001, desacopla servicios de backends
5. **Makefile simple y efectivo**: Ciclo de vida completo (up/down/restart/logs/clean)
6. **Multi-stage Dockerfiles**: Ambos servicios usan builder pattern para imagenes finales ligeras
7. **Rust Dockerfile con dependency caching**: El truco de `mkdir src && echo "fn main() {}" > src/main.rs && cargo build` cachea dependencias

### Incoherencias detectadas

| Elemento | Declaracion | Realidad |
|----------|-------------|----------|
| filter/business + filter/technical | Definidos en otelcol/config.yaml | No usados en ningun pipeline |
| Logs pipeline | Definido en otelcol (otlp -> loki) | Ningun servicio emite logs via OTLP |
| SLOs/Error Budgets | Prometidos en README | No implementados |
| Multi-language parity | "Ejemplos de buenas practicas" (README) | Rust solo tiene traces |
| Business metrics separation | ADR-0001 los describe | Todas las metricas van al mismo pipeline |

---

## 6. Plan de Accion Priorizado

### Fase 1 - Corregir Dead Config y Completar Pipeline (Critico)

1. **Activar filter processors** creando pipelines separados para metricas de negocio y tecnicas:
   ```yaml
   pipelines:
     metrics/business:
       receivers: [otlp]
       processors: [memory_limiter, filter/business, resource/env, batch]
       exporters: [prometheusremotewrite, logging]
     metrics/technical:
       receivers: [otlp, prometheus]
       processors: [memory_limiter, filter/technical, resource/env, batch]
       exporters: [prometheusremotewrite, logging]
   ```

2. **Agregar metricas al Rust service**: Implementar `opentelemetry-sdk` meter con contadores equivalentes (`business.orders.created`, `business.order.value`)

3. **Agregar exportacion de logs** a ambos servicios (Go: via OTel log bridge, Rust: via tracing-opentelemetry log bridge)

### Fase 2 - Hardening y Operabilidad (Alto)

4. **Reducir Grafana anonymous role** a `Viewer`
5. **Agregar health checks** a docker-compose para todos los backends
6. **Eliminar `version: '3.8'`** del docker-compose
7. **Instrumentar traffic generator del Rust service** con `reqwest-middleware` + `reqwest-tracing` o equivalente
8. **Actualizar Loki schema** a v13 con TSDB store
9. **Pinear alpine version** en Dockerfiles (e.g., `alpine:3.21`)

### Fase 3 - Dashboard y Observabilidad del Pipeline (Medio)

10. **Agregar scrape del OTel Collector** en prometheus.yml o en la propia config del Collector
11. **Documentar dependencia de Tempo span-metrics** en el dashboard RED/USE o generar las metricas desde el SDK
12. **Agregar SLO dashboard** con burn-rate panels como minimo
13. **Validar dashboards JSON en CI** (jq syntax check + UID cross-reference)

### Fase 4 - Polish (Bajo)

14. **Agregar .gitignore**
15. **Migrar `rand.Intn` a `rand.IntN`** en Go service
16. **Usar `OTEL_SERVICE_NAME` env var** en lugar de hardcoding
17. **Agregar `OTEL_RESOURCE_ATTRIBUTES`** en docker-compose para enrichment consistente
