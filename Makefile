SHELL := /bin/sh

COMPOSE := docker compose

.PHONY: up-core ingest api full prebuilt build-release logs down

up-core:
	$(COMPOSE) up -d qdrant ollama

ingest:
	$(COMPOSE) up -d qdrant
	$(COMPOSE) --profile ingest up --build -d ingest
	$(COMPOSE) logs -f ingest

api:
	$(COMPOSE) up -d --build api web

full:
	$(COMPOSE) --profile ingest up -d --build

prebuilt:
	$(COMPOSE) -f docker-compose.yml -f docker-compose.prebuilt.yml --profile prebuilt up -d

build-release:
	API_BUILD_TARGET=release-api INGEST_BUILD_TARGET=release-ingest $(COMPOSE) build api ingest

logs:
	$(COMPOSE) logs -f --tail=100 api

down:
	$(COMPOSE) down
