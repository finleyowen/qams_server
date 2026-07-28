-- Consolidated QAMS schema

CREATE TABLE IF NOT EXISTS scorecards (
    id                  BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name                VARCHAR(255)    NOT NULL,
    default_period_days INT UNSIGNED    NULL COMMENT 'Default reporting period length in days. NULL means no default.',
    created_at          DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS criteria (
    id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    scorecard_id BIGINT UNSIGNED NOT NULL,
    name         VARCHAR(255)    NOT NULL,
    position     INT UNSIGNED    NOT NULL DEFAULT 0,
    created_at   DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_criteria_scorecard
        FOREIGN KEY (scorecard_id) REFERENCES scorecards(id) ON DELETE CASCADE,
    UNIQUE KEY uq_criterion_name (scorecard_id, name),
    INDEX idx_criteria_scorecard (scorecard_id, position)
);

CREATE TABLE IF NOT EXISTS criterion_options (
    id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    criterion_id BIGINT UNSIGNED NOT NULL,
    name         VARCHAR(255)    NOT NULL,
    position     INT UNSIGNED    NOT NULL DEFAULT 0,
    score_type   ENUM('points', 'na', 'autofail') NOT NULL,
    points       INT UNSIGNED    NULL,
    created_at   DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_criterion_options_criterion
        FOREIGN KEY (criterion_id) REFERENCES criteria(id) ON DELETE CASCADE,
    UNIQUE KEY uq_option_name (criterion_id, name),
    INDEX idx_options_criterion (criterion_id, position)
);

CREATE TABLE IF NOT EXISTS agents (
    id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name       VARCHAR(255)    NOT NULL,
    metadata   JSON            NOT NULL DEFAULT ('{}'),
    created_at DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS reviews (
    id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    scorecard_id BIGINT UNSIGNED NOT NULL,
    agent_id     BIGINT UNSIGNED NOT NULL,
    reviewer     VARCHAR(255)    NOT NULL,
    date         DATE            NOT NULL,
    selections   JSON            NOT NULL,
    comments     JSON            NOT NULL DEFAULT ('{}'),
    score        DOUBLE          NOT NULL,
    adj_score    DOUBLE          NULL,
    created_at   DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_reviews_scorecard
        FOREIGN KEY (scorecard_id) REFERENCES scorecards(id),
    CONSTRAINT fk_reviews_agent
        FOREIGN KEY (agent_id) REFERENCES agents(id),
    INDEX idx_reviews_scorecard_date (scorecard_id, date)
);

CREATE TABLE IF NOT EXISTS reports (
    id                  BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    scorecard_id        BIGINT UNSIGNED NOT NULL,
    label               VARCHAR(255)    NOT NULL,
    start_date          DATE            NOT NULL,
    end_date            DATE            NOT NULL,
    accumulation_period INT UNSIGNED    NULL COMMENT 'Number of previous reports to include. NULL means all available.',
    created_at          DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_reports_scorecard
        FOREIGN KEY (scorecard_id) REFERENCES scorecards(id),
    INDEX idx_reports_scorecard_start (scorecard_id, start_date)
);
