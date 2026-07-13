-- Replace the flat CSV column on scorecards with normalised criteria/options tables.

ALTER TABLE scorecards DROP COLUMN csv;

CREATE TABLE IF NOT EXISTS criteria (
    id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    scorecard_id BIGINT UNSIGNED NOT NULL,
    name         VARCHAR(255)    NOT NULL,
    position     INT UNSIGNED    NOT NULL DEFAULT 0,
    created_at   DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (scorecard_id) REFERENCES scorecards(id) ON DELETE CASCADE,
    UNIQUE KEY uq_criterion_name (scorecard_id, name),
    INDEX idx_criteria_scorecard (scorecard_id, position)
);

CREATE TABLE IF NOT EXISTS criterion_options (
    id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    criterion_id BIGINT UNSIGNED NOT NULL,
    name         VARCHAR(255)    NOT NULL,
    position     INT UNSIGNED    NOT NULL DEFAULT 0,
    -- "points", "na", or "autofail"
    score_type   ENUM('points', 'na', 'autofail') NOT NULL,
    -- Only populated when score_type = 'points'
    points       INT UNSIGNED    NULL,
    created_at   DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (criterion_id) REFERENCES criteria(id) ON DELETE CASCADE,
    UNIQUE KEY uq_option_name (criterion_id, name),
    INDEX idx_options_criterion (criterion_id, position)
);