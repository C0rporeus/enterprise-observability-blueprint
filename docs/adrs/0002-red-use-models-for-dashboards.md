# 2. Adopción de los Modelos RED y USE para el Diseño de Dashboards

Date: 2026-02-28

## Status
Aceptado

## Contexto
Tener métricas disponibles no es suficiente si no se estructuran de una forma que responda a preguntas operativas críticas de manera rápida. Los dashboards desorganizados o sobrecargados con "todas las métricas que existen" complican el diagnóstico de incidentes durante las guardias (on-call).

## Decisión
Todos los dashboards técnicos para la observación de servicios se construirán siguiendo estrictamente dos modelos de la industria:

1. **El Modelo RED (para Servicios / Peticiones):**
   - **R**ate (Tasa): Número de peticiones por segundo.
   - **E**rrors (Errores): Número de peticiones fallidas.
   - **D**uration (Duración): Tiempo de procesamiento de las peticiones (Latencia p90, p95, p99).

2. **El Modelo USE (para Recursos de Infraestructura):**
   - **U**tilization (Utilización): Tiempo medio en que el recurso estuvo ocupado.
   - **S**aturation (Saturación): Medida de trabajo adicional encolado (queueing).
   - **E**rrors (Errores): Eventos de error del recurso.

## Consecuencias
- **Estandarización Visual:** Todos los servicios (independientemente del lenguaje en que estén escritos) mostrarán sus métricas clave bajo el mismo formato, facilitando a cualquier ingeniero entender rápidamente la salud de un servicio que no conoce profundamente.
- **Foco en el Usuario:** El modelo RED se centra en la experiencia real de la petición, que es lo que afecta directamente a clientes u otros servicios.
- **Reducción de Ruido:** Evitamos crear paneles de métricas poco relevantes en las vistas de alto nivel. Las métricas profundas o específicas de un runtime (ej. recolector de basura) quedarán relegadas a dashboards de "drill-down" detallado.