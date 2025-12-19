CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE businesses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 

CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    business_id  UUID NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    key_hash     TEXT NOT NULL,
    key_prefix   TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at   TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_api_keys_key_hash
    ON api_keys(key_hash);

CREATE INDEX idx_api_keys_business_id
    ON api_keys(business_id);

-- 

CREATE TABLE accounts (
    id           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    business_id  UUID NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    currency     CHAR(3) NOT NULL,
    balance      BIGINT NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT balance_non_negative CHECK (balance >= 0)
);

CREATE INDEX idx_accounts_business_id
    ON accounts(business_id);

-- 

CREATE TYPE transaction_type AS ENUM (
    'credit',
    'debit',
    'transfer'
);

CREATE TABLE transactions (
    id                UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    business_id       UUID NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    type              transaction_type NOT NULL,
    source_account_id UUID REFERENCES accounts(id),
    dest_account_id   UUID REFERENCES accounts(id),
    amount            BIGINT NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT amount_positive CHECK (amount > 0),

    CONSTRAINT valid_transaction_accounts CHECK (
        (type = 'credit'   AND source_account_id IS NULL AND dest_account_id IS NOT NULL) OR
        (type = 'debit'    AND source_account_id IS NOT NULL AND dest_account_id IS NULL) OR
        (type = 'transfer' AND source_account_id IS NOT NULL AND dest_account_id IS NOT NULL)
    )
);

CREATE INDEX idx_transactions_business_id
    ON transactions(business_id);

CREATE INDEX idx_transactions_source_account
    ON transactions(source_account_id);

CREATE INDEX idx_transactions_dest_account
    ON transactions(dest_account_id);

-- 

CREATE TABLE webhook_endpoints (
    id           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    business_id  UUID NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    url          TEXT NOT NULL,
    secret       TEXT NOT NULL,
    active       BOOLEAN NOT NULL DEFAULT true,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_webhook_endpoints_business_id
    ON webhook_endpoints(business_id);

-- 

CREATE TYPE webhook_status AS ENUM (
    'pending',
    'delivered',
    'failed'
);

CREATE TABLE webhook_events (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    endpoint_id     UUID NOT NULL REFERENCES webhook_endpoints(id) ON DELETE CASCADE,
    transaction_id  UUID NOT NULL REFERENCES transactions(id),
    payload         JSONB NOT NULL,
    status          webhook_status NOT NULL DEFAULT 'pending',
    attempts        INT NOT NULL DEFAULT 0,
    next_retry_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_webhook_events_status
    ON webhook_events(status);

CREATE INDEX idx_webhook_events_next_retry
    ON webhook_events(next_retry_at);