# 3. Uso de Span-Derived Metrics para Dashboards RED

Date: 2026-03-01

## Status
Aceptado

## Contexto
Para implementar el modelo RED (Rate, Errors, Duration), se necesitan métricas HTTP como `http_server_duration_count` y `http_server_duration_bucket`. Estas pueden obtenerse de dos formas:

1. **Desde el SDK de cada servicio:** Cada aplicación emite métricas HTTP explícitamente usando el SDK de OTel (ej. `otelhttp` en Go, middleware en Rust).
2. **Desde Tempo (span-metrics generator):** Tempo analiza las trazas recibidas y genera métricas derivadas automáticamente a partir de los spans.

## Decisión
Adoptamos la **opción 2**: las métricas RED del dashboard técnico se derivan de las trazas a través del **span-metrics generator de Tempo**. Tempo genera `http_server_duration_count`, `http_server_duration_bucket` y `http_server_duration_sum` a partir de los spans recibidos, y las envía a Prometheus vía `remote_write`.

Las métricas de negocio (`business.orders.created`, `business.order.value`) siguen emitiéndose directamente desde los SDKs de cada servicio, ya que representan lógica de dominio que no puede derivarse de spans genéricos.

## Consecuencias
- **Dependencia implícita:** El dashboard RED requiere que Tempo tenga `metrics_generator` habilitado con `remote_write` hacia Prometheus. Si Tempo falla o se desactiva el generador, el dashboard RED se queda sin datos.
- **Menor instrumentación en el código:** Los servicios no necesitan emitir métricas HTTP explícitas; solo necesitan emitir trazas correctamente instrumentadas.
- **Consistencia:** Las métricas RED reflejan exactamente lo que muestran las trazas, eliminando discrepancias entre ambas señales.
- **Latencia:** Las métricas derivadas tienen un ligero retraso adicional respecto a métricas emitidas directamente por el SDK.
