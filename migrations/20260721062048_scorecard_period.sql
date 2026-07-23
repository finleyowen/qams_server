-- Add migration script here
ALTER TABLE scorecards
  ADD COLUMN default_period_days INT UNSIGNED NULL
    COMMENT 'Default reporting period length in days. NULL means no default.';