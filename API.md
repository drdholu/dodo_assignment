### API Documentation

### Base URL

- Local: `http://localhost:3000`

All authenticated endpoints are under **`/api`**.

### Authentication

- **Header**: `X-API-Key: <raw_api_key>`
- **Missing or invalid key**: `401` with body `{"error":"Unauthorized"}`

The server derives a hash using `HMAC_SECRET` and matches it against `api_keys.key_hash` in Postgres.

### Common error format

Most errors return JSON:

- `{"error":"Unauthorized"}` (401)
- `{"error":"Data Not found"}` (404)
- `{"error":"Internal Sevrer error"}` (500)

Some validation errors return `400` with a more specific message, e.g. `{"error":"amount must be > 0"}`.

### Health (no auth)

#### `GET /health`

Response `200`:

```json
{ "status": "OK", "message": "Server is running" }
```

#### `GET /health/db`

Response `200`:

```json
{ "status": "OK", "message": "DB reachable" }
```

### Accounts (auth required)

#### `POST /api/create-account`

Create an account for the authenticated business.

Request JSON:

```json
{ "name": "primary", "currency": "USD" }
```

Rules:

- `name`: required, 1–128 chars, unique per business
- `currency`: required, 3 ASCII letters; normalized to uppercase

Response `201`:

```json
{ "id":"<uuid>", "name":"primary", "currency":"USD", "balance":0 }
```

Errors:

- `400` if invalid name/currency
- `409` if account name already exists for this business

#### `GET /api/accounts`

List accounts for the authenticated business.

Response `200`:

```json
[
  { "id":"<uuid>", "name":"primary", "currency":"USD", "balance":0 }
]
```

#### `GET /api/accounts/{id}`

Get an account by id (must belong to the authenticated business).

Response `200`:

```json
{ "id":"<uuid>", "name":"primary", "currency":"USD", "balance":0 }
```

Errors:

- `404` if not found (or not owned by the business)

### Transactions (auth required)

#### `POST /api/transactions`

Create a transaction and update balances atomically.

Request JSON (union by `type`):

- Credit:

```json
{ "type":"credit", "amount": 1000, "dest_account_id":"<uuid>" }
```

- Debit:

```json
{ "type":"debit", "amount": 250, "source_account_id":"<uuid>" }
```

- Transfer:

```json
{
  "type":"transfer",
  "amount": 500,
  "source_account_id":"<uuid>",
  "dest_account_id":"<uuid>"
}
```

Rules:

- `amount` must be `> 0`
- Credit: requires `dest_account_id` and **no** `source_account_id`
- Debit: requires `source_account_id` and **no** `dest_account_id`
- Transfer: requires both account ids, must be distinct, and must have the same currency
- Debit/transfer require sufficient funds

Response `201`:

```json
{
  "id":"<uuid>",
  "type":"credit",
  "source_account_id": null,
  "dest_account_id":"<uuid>",
  "amount": 1000,
  "created_at":"2025-12-21T00:00:00Z"
}
```

Errors:

- `400` for validation failures (including insufficient funds)
- `404` if referenced account(s) are not found / not owned by the business

#### `GET /api/transactions`

List recent transactions for the authenticated business (up to 100).

Response `200`:

```json
[
  {
    "id":"<uuid>",
    "type":"transfer",
    "source_account_id":"<uuid>",
    "dest_account_id":"<uuid>",
    "amount": 500,
    "created_at":"2025-12-21T00:00:00Z"
  }
]
```

#### `GET /api/transactions/{id}`

Get a transaction by id (must belong to the authenticated business).

Response `200`: same shape as list items.

Errors:

- `404` if not found (or not owned by the business)

### Webhooks (auth required)

#### `POST /api/webhooks`

Register a webhook endpoint URL for the authenticated business.

Request JSON:

```json
{ "url":"https://example.com/webhooks/receiver" }
```

Rules:

- `url` is required
- must start with `http://` or `https://`

Response `201`:

```json
{
  "id":"<uuid>",
  "url":"https://example.com/webhooks/receiver",
  "active": true,
  "created_at":"2025-12-21T00:00:00Z"
}
```

#### `GET /api/webhooks`

List webhook endpoints for the authenticated business.

Response `200`:

```json
[
  {
    "id":"<uuid>",
    "url":"https://example.com/webhooks/receiver",
    "active": true,
    "created_at":"2025-12-21T00:00:00Z"
  }
]
```

#### `DELETE /api/webhooks/{id}`

Deactivate a webhook endpoint (soft delete).

Response `204 No Content` on success.

Errors:

- `404` if not found (or not owned by the business)

### Webhook delivery behavior

When a transaction is created, the service enqueues an event for each active endpoint for that business and a background worker attempts delivery.

- **Method**: `POST`
- **Headers**: `Content-Type: application/json`
- **Body**: JSON payload (example):

```json
{
  "event_type": "transaction.created",
  "timestamp": "2025-12-21T00:00:00Z",
  "data": {
    "transaction_id": "<uuid>",
    "type": "credit",
    "source_account_id": null,
    "dest_account_id": "<uuid>",
    "amount": 1000,
    "created_at": "2025-12-21T00:00:00Z"
  }
}
```

Retries:

- Worker polls every ~2s, batch size 25.
- On failure: exponential backoff (up to 5 minutes), max 5 attempts, then marks event failed.

Security note:

- Webhook signing is **not implemented yet** (no signature headers, endpoint `secret` is unused). See `DESIGN.md` for recommended next steps.


