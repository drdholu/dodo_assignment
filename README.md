# dodo_assign

Minimal transaction service (Rust + Axum + Postgres).

**Note:** AI coding assistants were used during development to help with Rust syntax and implementation patterns, as the author is relatively new to Rust.

## Run with Docker (one-command)

From the repo root:

### create env file

First time only (create `.env`):

```bash
cp .env.example .env
```

### docker compose

```bash
docker compose up --build
```


### inserting test business in db


1. `\set hmac_secret 'super-secret-string'` (check .env for hmac_secret)


2. run inside the psql shell

```sql
BEGIN;

-- Needed for hmac(); safe to run multiple times
CREATE EXTENSION IF NOT EXISTS pgcrypto;

WITH new_business AS (
  INSERT INTO businesses (name)
  VALUES ('Acme Test Business')
  RETURNING id
),
new_api_key AS (
  INSERT INTO api_keys (business_id, key_hash, key_prefix)
  SELECT
    b.id,
    encode(hmac('dk_test_1234567890abcdef', :'hmac_secret', 'sha256'), 'hex'),
    substring('dk_test_1234567890abcdef' from 1 for 8)
  FROM new_business b
  RETURNING id, business_id, key_prefix
)
SELECT
  b.id          AS business_id,
  k.id          AS api_key_id,
  k.key_prefix  AS key_prefix
FROM new_business b
JOIN new_api_key k ON k.business_id = b.id;

COMMIT;
```


> The API listens on `http://localhost:3000`.

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

## Docs

- `DESIGN.md`: design decisions, schema, reliability/security notes
- `API.md`: HTTP API reference (endpoints, schemas, examples)

