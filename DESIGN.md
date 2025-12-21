### Design: Simple Transaction Service (Rust + Axum + Postgres)

### Goals (per assignment)

- **API authentication**: secure access with API keys.
- **Accounts**: create accounts for a business and read balances.
- **Transactions**: record `credit`, `debit`, `transfer` and update balances **atomically**.
- **Webhooks**: notify businesses about transactions **reliably and securely**.
- **Database**: relational DB to persist all state (Postgres).
- **Docker Compose**: one-command local setup.

This repo implements the core flows end-to-end; see **“Gaps / next steps”** for the security items that are intentionally not implemented yet (notably webhook signing).

### Key assumptions

- One business owns many accounts; every request is scoped to exactly one business via the API key.
- Amounts are stored as **integer minor units** (`BIGINT`), e.g. cents.
- Accounts have a single `currency` (3-letter code like `USD`); transfers require same currency.
- Balances never go negative (enforced by transaction logic and a DB check constraint).

### High-level architecture

- **HTTP API**: Axum routes under `/api/*` protected by API key middleware.
- **Postgres**: stores businesses, API keys, accounts, transactions, webhook endpoints, webhook events.
- **Webhook worker**: background task in the same process that polls `webhook_events` and POSTs JSON to endpoints with retries.

### Authentication model

- Clients send `X-API-Key: <raw_api_key>`.
- Server computes `HMAC-SHA256(secret = HMAC_SECRET, message = raw_api_key)` and hex-encodes it.
- DB stores only the hash (`api_keys.key_hash`) + a prefix (`api_keys.key_prefix`) for human debugging.
- Revocation is supported by setting `api_keys.revoked_at`; only keys with `revoked_at IS NULL` authorize.

**Security properties**

- Raw API keys are never stored in the database.
- Rotating `HMAC_SECRET` would invalidate all keys (because it changes the derived hash); a rotation strategy would require dual secrets or per-key salts.

### Data model (schema overview)

Defined in `migrations/001_init_schema.sql` and `migrations/002_add_account_name.sql`.

- **`businesses`**: tenant boundary.
  - `id`, `name`, `created_at`
- **`api_keys`**: access credentials.
  - `business_id`, `key_hash` (unique), `key_prefix`, `revoked_at`
- **`accounts`**: per-business balances.
  - `business_id`, `name` (unique per business), `currency` (`CHAR(3)`), `balance` (`BIGINT`)
  - DB check: `balance >= 0`
- **`transactions`**: immutable money movements.
  - `type` enum: `credit | debit | transfer`
  - `source_account_id` / `dest_account_id` constraints enforced in DB (`valid_transaction_accounts`)
  - DB check: `amount > 0`
- **`webhook_endpoints`**: per-business destinations.
  - `url`, `active`, `secret` (currently unused; see gaps)
- **`webhook_events`**: outbox queue.
  - `endpoint_id`, `transaction_id`, `payload` (`JSONB`)
  - `status` enum: `pending | delivered | failed`
  - retry fields: `attempts`, `next_retry_at`

### Transaction processing & atomic balance updates

All transaction creation is implemented in `src/services/transaction_service.rs` and is designed around **a single Postgres transaction**:

- Start DB transaction (`BEGIN`).
- `SELECT ... FOR UPDATE` to lock involved account row(s).
  - Transfer uses a deterministic lock order (by UUID bytes) to reduce deadlock risk.
- Validate business ownership (by scoping every `SELECT` by `business_id`).
- Enforce rules:
  - `amount > 0`
  - `credit`: only destination account
  - `debit`: only source account + sufficient funds
  - `transfer`: both accounts + distinct + same currency + sufficient funds
- Update balance(s) and insert a `transactions` row within the same DB transaction.
- Commit.

**Result**: balances and the transaction record are updated **atomically**.

### Webhook design (reliability)

This implementation uses an **outbox table** (`webhook_events`) plus a background worker:

- After a transaction is successfully committed, the service *best-effort* enqueues a `"transaction.created"` event into `webhook_events` for every active endpoint for that business.
- Worker (`src/worker/webhook_worker.rs`) polls every 2 seconds:
  - fetches up to 25 due events (`status = pending` and `next_retry_at <= now()`),
  - sends `POST` with `Content-Type: application/json`,
  - on HTTP 2xx: marks delivered,
  - on non-2xx / network error: increments attempts and schedules `next_retry_at` using exponential backoff (capped at 5 minutes),
  - after 5 attempts: marks event `failed` (terminal).

**Important trade-off (current implementation)**: enqueue is performed **after** committing the money movement, and failures to enqueue are logged but do not affect the API response. This means:

- Money movement is correct and durable.
- Webhook delivery is “reliable” only insofar as the enqueue step succeeds.

To make delivery truly robust, enqueue should be part of the same DB transaction (or use a separate durable queue).

### Webhook security (current behavior)

Today, webhooks have **no signing and no secret distribution**:

- `webhook_endpoints.secret` exists in schema but is currently stored as an empty string and not used.
- Requests contain only JSON body + `Content-Type` header.

### Operational considerations

- **Migrations** run on startup (`sqlx::migrate!("./migrations")`).
- **Readiness**: `/health` (process up) and `/health/db` (DB reachable).
- **Connection pooling**: `PgPoolOptions` with startup retry loop (docker-compose friendliness).
- **Single-process**: HTTP server + webhook worker are in the same binary process.

### Gaps / next steps (explicit)

- **Webhook signing + secret management (security requirement)**:
  - Generate a per-endpoint secret at creation time.
  - Sign payloads (e.g., `HMAC-SHA256(secret, raw_body)`), include headers like `X-Webhook-Signature` and `X-Webhook-Timestamp`.
  - Recommend receiver-side verification and replay protection.
- **Durable outbox enqueue**:
  - Insert webhook events inside the same DB transaction as the `transactions` insert (true transactional outbox).
  - Add `FOR UPDATE SKIP LOCKED` to `fetch_due_webhook_events` to avoid duplicate delivery if multiple workers are added.
- **Idempotency** (bonus):
  - support `Idempotency-Key` header for POST endpoints, persist request hashes + resulting transaction IDs.
- **Rate limiting** (bonus):
  - per API key token bucket (in-memory or Redis).
- **Observability** (bonus):
  - structured logging + OpenTelemetry spans and metrics for enqueue/delivery and retry outcomes.


