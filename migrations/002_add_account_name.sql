-- add account display name for easier identification.
-- name is required and unique per business.

ALTER TABLE accounts
ADD COLUMN name TEXT;

UPDATE accounts
SET name = id::text
WHERE name IS NULL;

ALTER TABLE accounts
ALTER COLUMN name SET NOT NULL;

CREATE UNIQUE INDEX idx_accounts_business_id_name
ON accounts (business_id, name);


