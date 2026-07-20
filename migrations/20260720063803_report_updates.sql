-- Add migration script here
ALTER TABLE reports
  ADD COLUMN accumulation_period INT UNSIGNED NULL
    COMMENT 'Number of previous reports to include. NULL means all available.';