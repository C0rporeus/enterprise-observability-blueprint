# Enterprise Observability Blueprint

Una arquitectura de referencia de grado empresarial que demuestra patrones avanzados de telemetria y observabilidad utilizando OpenTelemetry, Prometheus, Loki, Tempo y Grafana.

## Objetivo

El proposito de este repositorio es ir mas alla del tipico "Hola Mundo" en observabilidad. Se enfoca en:

- **Pipelines Custom de OpenTelemetry:** Separacion y procesamiento estructurado de la telemetria con pipelines diferenciados para metricas de negocio vs tecnicas.
- **Metricas con Significado:** Diferenciacion entre metricas de negocio (`business.*`) y metricas tecnicas, cada una con su propio pipeline en el Collector.
- **Modelos Estrategicos:** Implementacion de dashboards basados en los modelos **USE** (Utilization, Saturation, Errors) y **RED** (Rate, Errors, Duration).
- **Correlacion Cross-Signal:** Navegacion fluida entre Logs, Trazas y Metricas a traves de Grafana con exemplars y derived fields.
- **Instrumentacion Multi-lenguaje:** Ejemplos de buenas practicas emitiendo trazas, metricas y logs desde Go y Rust con paridad de senales.
- **Observabilidad del Pipeline:** Dashboard dedicado a la salud interna del OTel Collector (throughput, errores de exportacion, colas).

## Arquitectura

```
┌────────────┐    ┌──────────────┐
│ Go Service │    │ Rust Service  │
│ :8080      │    │ :8081         │
│ traces +   │    │ traces +      │
│ metrics +  │    │ metrics +     │
│ logs       │    │ logs          │
└─────┬──────┘    └──────┬────────┘
      │ OTLP gRPC        │ OTLP gRPC
      └────────┬──────────┘
               ▼
     ┌──────────────────────┐
     │   OTel Collector      │
     │   :4317 (gRPC)        │
     │   :4318 (HTTP)        │
     │   :8888 (self-metrics)│
     │                       │
     │  Pipelines:           │
     │  ├─ metrics/business  │  ← filter: business.*
     │  ├─ metrics/technical │  ← filter: !business.*
     │  ├─ traces            │
     │  └─ logs              │
     └──┬────┬────┬──────────┘
        │    │    │
        ▼    ▼    ▼
     Prom  Loki  Tempo
     :9090 :3100 :3200
        │    │    │  ↑ remote_write
        │    │    │  │ (span-metrics)
        │    │    └──┘
        └────┼────┘
             ▼
       Grafana :3000
       ├─ RED & USE Service Metrics
       ├─ Business Metrics Overview
       └─ Pipeline Health (OTel Collector)
```

### Componentes

| Componente | Puerto | Rol |
|---|---|---|
| Go Service | `:8080` | Servicio demo instrumentado (Go + OTel SDK) |
| Rust Service | `:8081` | Servicio demo instrumentado (Rust + OTel SDK) |
| OTel Collector | `:4317` `:4318` | Punto unico de ingesta OTLP, enrutamiento y procesamiento |
| Prometheus | `:9090` | Almacenamiento de metricas con exemplar storage |
| Loki | `:3100` | Agregacion de logs con derived fields para trace correlation |
| Tempo | `:3200` | Backend de trazas con span-metrics generator |
| Grafana | `:3000` | Visualizacion unificada con 3 dashboards provisionados |

### Correlacion Cross-Signal

```
Metrics ──exemplars──► Traces
Logs ──derived fields──► Traces
Traces ──tracesToLogsV2──► Logs
```

## Empezar Rapidamente

```bash
# Levantar todo el stack
make up

# Ver logs de la infraestructura
make logs

# Apagar el stack
make down

# Reiniciar
make restart

# Limpieza completa (incluye docker prune)
make clean
```

Una vez levantado:
- **Grafana:** [http://localhost:3000](http://localhost:3000) (acceso anonimo como Viewer)
- **Prometheus:** [http://localhost:9090](http://localhost:9090)
- **Tempo:** [http://localhost:3200](http://localhost:3200)

Los servicios generan trafico automaticamente cada segundo. En ~30 segundos los dashboards empiezan a mostrar datos.

## Dashboards

| Dashboard | Contenido |
|---|---|
| **RED & USE Service Metrics** | Rate, Errors, Duration (RED) + Memory, CPU (USE) + Service Logs. Metricas RED derivadas de Tempo span-metrics. |
| **Business Metrics Overview** | KPIs de negocio: orders created rate, total revenue simulado. Metricas emitidas directamente por los SDKs. |
| **Pipeline Health** | Salud del OTel Collector: spans/metrics/logs recibidos, errores de exportacion, tamano de colas, uso de CPU/memoria. |

## Documentacion y Decisiones

| ADR | Decision |
|---|---|
| [ADR-0001](docs/adrs/0001-use-opentelemetry-as-standard.md) | OpenTelemetry como estandar universal de telemetria |
| [ADR-0002](docs/adrs/0002-red-use-models-for-dashboards.md) | Modelos RED y USE para diseno de dashboards |
| [ADR-0003](docs/adrs/0003-span-derived-metrics-for-red-dashboards.md) | Span-derived metrics para dashboards RED |

## Troubleshooting

**Los dashboards no muestran datos:**
1. Verificar que todos los servicios estan healthy: `docker compose -f deploy/docker-compose/docker-compose.yaml ps`
2. Revisar el dashboard "Pipeline Health" para errores de exportacion
3. Confirmar que Prometheus recibe metricas: [http://localhost:9090/targets](http://localhost:9090/targets)

**El dashboard RED esta vacio:**
- Las metricas RED (`http_server_duration_*`) son generadas por Tempo, no por los SDKs. Verificar que Tempo tiene `metrics_generator` habilitado y `remote_write` configurado.

**Logs no aparecen en Grafana:**
- Verificar que el pipeline de logs del Collector esta activo y que Loki esta ready: `curl http://localhost:3100/ready`

---

*Este proyecto es parte del portafolio tecnico de Platform Engineering, Observabilidad y Arquitectura Cloud.*
