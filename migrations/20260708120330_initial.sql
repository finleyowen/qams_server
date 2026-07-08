CREATE TABLE IF NOT EXISTS scorecards (
    id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name       VARCHAR(255)    NOT NULL,
    csv        MEDIUMTEXT      NOT NULL,
    created_at DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS agents (
    id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name       VARCHAR(255)    NOT NULL,
    -- Arbitrary JSON metadata (e.g. team, role, start date)
    metadata   JSON            NOT NULL DEFAULT ('{}'),
    created_at DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS reviews (
    id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    scorecard_id BIGINT UNSIGNED NOT NULL REFERENCES scorecards(id),
    agent_id     BIGINT UNSIGNED NOT NULL REFERENCES agents(id),
    reviewer     VARCHAR(255)    NOT NULL,
    date         DATE            NOT NULL,
    -- JSON object: criterion name → selected option name
    selections   JSON            NOT NULL,
    -- JSON object: criterion name → comment string
    comments     JSON            NOT NULL DEFAULT ('{}'),
    score        DOUBLE          NOT NULL,
    adj_score    DOUBLE          NULL,
    created_at   DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_reviews_scorecard_date (scorecard_id, date)
);

CREATE TABLE IF NOT EXISTS reports (
    id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    scorecard_id BIGINT UNSIGNED NOT NULL REFERENCES scorecards(id),
    label        VARCHAR(255)    NOT NULL,
    start_date   DATE            NOT NULL,
    end_date     DATE            NOT NULL,
    created_at   DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_reports_scorecard_start (scorecard_id, start_date)
);
