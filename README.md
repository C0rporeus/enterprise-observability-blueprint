# 🔷 Enterprise Observability Blueprint

Una arquitectura de referencia de grado empresarial que demuestra patrones avanzados de telemetría y observabilidad utilizando OpenTelemetry, Prometheus, Loki, Tempo y Grafana.

## 🎯 Objetivo

El propósito de este repositorio es ir más allá del típico "Hola Mundo" en observabilidad. Se enfoca en:

- **Pipelines Custom de OpenTelemetry:** Separación y procesamiento estructurado de la telemetría.
- **Métricas con Significado:** Diferenciación entre métricas de negocio y métricas técnicas.
- **Modelos Estratégicos:** Implementación de dashboards basados en los modelos **USE** (Utilization, Saturation, Errors) y **RED** (Rate, Errors, Duration).
- **SLOs y Error Budgets:** Visualización del nivel de servicio real percibido por el usuario.
- **Instrumentación Multi-lenguaje:** Ejemplos de buenas prácticas emitiendo trazas y métricas desde Go, Rust, etc.

## 🏗️ Arquitectura

La pila tecnológica central incluye:

- **OpenTelemetry Collector**: Enrutador y procesador universal de señales (Trazas, Métricas, Logs).
- **Prometheus**: Almacenamiento optimizado de métricas.
- **Loki**: Agregación de logs con foco en etiquetas (labels).
- **Tempo**: Backend de trazabilidad distribuida de alta capacidad.
- **Grafana**: Capa de visualización unificada y aprovisionada automáticamente.
- **Servicios Instrumentados**: Aplicaciones demo generando telemetría real (en Go, Rust, etc.).

## 🚀 Empezar Rápidamente

Todo el stack de observabilidad está definido en Docker Compose. Para levantarlo:

```bash
make up
```

Para apagar la infraestructura de forma limpia:

```bash
make down
```

Para ver los logs del colector o de los servicios:

```bash
make logs
```

## 📚 Documentación y Decisiones

Revisa la carpeta `docs/adrs/` para entender las decisiones arquitectónicas que respaldan este proyecto (Architecture Decision Records).

---

*Este proyecto es parte del portafolio técnico de Platform Engineering, Observabilidad y Arquitectura Cloud.*
