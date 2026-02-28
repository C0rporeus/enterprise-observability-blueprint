# 1. Adopción de OpenTelemetry como Estándar Universal de Telemetría

Date: 2026-02-28

## Status
Aceptado

## Contexto
En un ecosistema moderno con múltiples lenguajes de programación y herramientas de backend (Prometheus, Loki, Tempo, Jaeger, etc.), la instrumentación y recolección de datos de observabilidad puede fragmentarse rápidamente. Históricamente, cada herramienta venía con sus propios SDKs y agentes, provocando alto acoplamiento en el código aplicativo.

## Decisión
Hemos decidido adoptar **OpenTelemetry (OTel)** como el estándar universal y exclusivo para la instrumentación de todos los servicios.
Adicionalmente, implementaremos el **OpenTelemetry Collector** como un punto único de ingesta que recibirá todas las señales (Trazas, Métricas y Logs).

Dentro del Collector, configuraremos **Pipelines Custom** que nos permitirán:
1. Enriquecer señales globalmente (ej. añadir etiquetas de entorno de despliegue `deployment.environment`).
2. Filtrar y separar "Métricas de Negocio" de "Métricas Técnicas", enviándolas a sistemas de almacenamiento o retenciones diferentes si fuera necesario.

## Consecuencias
- **Desacoplamiento:** El código de la aplicación ya no necesita saber qué backend almacena la información. Solo emite hacia el OTel Collector a través de OTLP (OpenTelemetry Protocol).
- **Flexibilidad:** Podemos cambiar de backend (ej. pasar de Tempo a Jaeger, o de Prometheus a Datadog) alterando únicamente la configuración del Collector, sin modificar una sola línea de código en las aplicaciones.
- **Curva de Aprendizaje:** Los equipos de desarrollo deben aprender a utilizar las librerías de OpenTelemetry (SDKs) para sus respectivos lenguajes (Go, Rust, Java, etc.).