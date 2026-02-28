.PHONY: up down restart logs clean

# Variables de entorno por defecto
DOCKER_COMPOSE_FILE := deploy/docker-compose/docker-compose.yaml

up:
	@echo "Levantando el stack de observabilidad..."
	docker compose -f $(DOCKER_COMPOSE_FILE) up -d

down:
	@echo "Apagando el stack de observabilidad..."
	docker compose -f $(DOCKER_COMPOSE_FILE) down

restart: down up

logs:
	@echo "Mostrando logs de la infraestructura (Ctrl+C para salir)..."
	docker compose -f $(DOCKER_COMPOSE_FILE) logs -f

clean: down
	@echo "Limpiando recursos huérfanos de Docker..."
	docker system prune -f
