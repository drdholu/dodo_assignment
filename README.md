# dodo_assign

Minimal transaction service (Rust + Axum + Postgres).

## Run with Docker (one-command)

From the repo root:

First time only (create `.env`):

```bash
cp .env.example .env
```

```bash
docker compose up --build
```

The API listens on `http://localhost:3000`.

## Configuration

Required environment variables (set in `.env` file):
- `DATABASE_URL` - Postgres connection string
- `HMAC_SECRET` - Secret for API key hashing
- `SERVER_PORT` - HTTP server port (defaults to `3000` if not set)

**Important: DATABASE_URL depends on where the app runs:**

- **Docker (app container → postgres container)**: Use `postgresql://appuser:apppassword@postgres:5432/appdb`
  - `postgres` = service name in docker-compose.yml
  - `5432` = internal Postgres port
- **Host machine (cargo run → postgres container)**: Use `postgresql://appuser:apppassword@localhost:5433/appdb`
  - `localhost` = host machine
  - `5433` = host-mapped port (from docker-compose.yml)

Docker Compose loads these from the `.env` file. For local (non-docker) runs, create a `.env` using the values in `.env.example`.

## Quick checks

Health:

```bash
curl -sS "http://localhost:3000/health"
curl -sS "http://localhost:3000/health/db"
```

See `plans/creds&cmd.md` for auth setup + curl examples (accounts, transactions, webhooks).

## Docs

- `DESIGN.md`: design decisions, schema, reliability/security notes
- `API.md`: HTTP API reference (endpoints, schemas, examples)


