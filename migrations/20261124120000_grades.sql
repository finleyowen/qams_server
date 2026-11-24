-- Named score bands per scorecard, used to colour scores in reports.
CREATE TABLE IF NOT EXISTS grades (
    id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    scorecard_id BIGINT UNSIGNED NOT NULL,
    name         VARCHAR(255)    NOT NULL,
    -- Minimum score (percentage, 0–100) a score must reach to get this grade
    min_score    DOUBLE          NOT NULL,
    -- CSS hex colours, e.g. '#16a34a'
    bg_color     CHAR(7)         NOT NULL,
    text_color   CHAR(7)         NOT NULL,
    created_at   DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (scorecard_id) REFERENCES scorecards(id) ON DELETE CASCADE,
    UNIQUE KEY uq_grade_name (scorecard_id, name),
    INDEX idx_grades_scorecard (scorecard_id, min_score)
);
