.PHONY: up down restart logs ps validate test test-backend test-frontend reset clean infra

up:            ## Build and start all services
	docker compose up -d --build

down:          ## Stop all services (data is kept)
	docker compose down

restart:
	docker compose restart backend frontend

logs:          ## Follow logs (2FA codes appear in backend logs)
	docker compose logs -f backend frontend

ps:
	docker compose ps

infra:         ## Start only Postgres + Redis (for running API/UI locally)
	docker compose up -d postgres redis

validate:      ## Run the 11-step validation workflow against the API
	./scripts/validate.sh

test: test-backend test-frontend

test-backend:
	docker compose --profile test run --rm backend-tests

test-frontend:
	docker compose --profile test run --rm frontend-tests

clean:         ## Stop everything and delete volumes (wipes the database)
	docker compose down -v
