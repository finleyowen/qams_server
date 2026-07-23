-- ============================================================
-- QAMS sample data
-- Run after all migrations have been applied.
-- Safe to re-run: uses INSERT IGNORE / conditional inserts
-- where possible, but easiest on a fresh database.
-- ============================================================

-- ── Scorecard ────────────────────────────────────────────────
INSERT INTO scorecards (id, name, default_period_days) VALUES
  (1, 'Voice QA v1', 7);

-- ── Criteria ─────────────────────────────────────────────────
INSERT INTO criteria (id, scorecard_id, name, position) VALUES
  (1, 1, 'Greeting',     0),
  (2, 1, 'Compliance',   1),
  (3, 1, 'Resolution',   2),
  (4, 1, 'Tone',         3);

-- ── Criterion options ─────────────────────────────────────────
-- Greeting: YES(1) / NO(0) / N/A
INSERT INTO criterion_options (criterion_id, name, position, score_type, points) VALUES
  (1, 'YES', 0, 'points', 1),
  (1, 'NO',  1, 'points', 0),
  (1, 'N/A', 2, 'na',     NULL);

-- Compliance: YES(1) / NO — autofail / N/A
INSERT INTO criterion_options (criterion_id, name, position, score_type, points) VALUES
  (2, 'YES', 0, 'points',   1),
  (2, 'NO',  1, 'autofail', NULL),
  (2, 'N/A', 2, 'na',       NULL);

-- Resolution: GREAT(3) / GOOD(2) / POOR(1) / N/A
INSERT INTO criterion_options (criterion_id, name, position, score_type, points) VALUES
  (3, 'GREAT', 0, 'points', 3),
  (3, 'GOOD',  1, 'points', 2),
  (3, 'POOR',  2, 'points', 1),
  (3, 'N/A',   3, 'na',     NULL);

-- Tone: YES(1) / NO(0)
INSERT INTO criterion_options (criterion_id, name, position, score_type, points) VALUES
  (4, 'YES', 0, 'points', 1),
  (4, 'NO',  1, 'points', 0);

-- ── Agents ───────────────────────────────────────────────────
INSERT INTO agents (id, name, metadata) VALUES
  (1, 'Alice Nguyen', '{"team": "Team A", "start_date": "2025-01-15"}'),
  (2, 'Ben Carter',   '{"team": "Team B", "start_date": "2025-03-01"}'),
  (3, 'Clara Singh',  '{"team": "Team A", "start_date": "2025-06-10"}');

-- ── Reviews — Week 1 (2026-07-07 to 2026-07-13) ──────────────
-- Alice: perfect score
INSERT INTO reviews (scorecard_id, agent_id, reviewer, date, selections, comments, score) VALUES
  (1, 1, 'QA Officer', '2026-07-09',
   '{"Greeting":"YES","Compliance":"YES","Resolution":"GREAT","Tone":"YES"}',
   '{}',
   1.0);

-- Ben: missed greeting, good resolution
INSERT INTO reviews (scorecard_id, agent_id, reviewer, date, selections, comments, score) VALUES
  (1, 2, 'QA Officer', '2026-07-10',
   '{"Greeting":"NO","Compliance":"YES","Resolution":"GOOD","Tone":"YES"}',
   '{"Greeting":"Did not introduce himself"}',
   0.8333);

-- Clara: N/A on compliance, poor resolution
INSERT INTO reviews (scorecard_id, agent_id, reviewer, date, selections, comments, score) VALUES
  (1, 3, 'QA Officer', '2026-07-11',
   '{"Greeting":"YES","Compliance":"N/A","Resolution":"POOR","Tone":"NO"}',
   '{"Resolution":"Customer issue was not resolved","Tone":"Sounded impatient"}',
   0.4);

-- Ben: second review, autofail on compliance
INSERT INTO reviews (scorecard_id, agent_id, reviewer, date, selections, comments, score) VALUES
  (1, 2, 'QA Officer', '2026-07-12',
   '{"Greeting":"YES","Compliance":"NO","Resolution":"GOOD","Tone":"YES"}',
   '{"Compliance":"Failed to read mandatory disclosure"}',
   0.0);

-- ── Reviews — Week 2 (2026-07-14 to 2026-07-20) ──────────────
-- Alice: near-perfect
INSERT INTO reviews (scorecard_id, agent_id, reviewer, date, selections, comments, score) VALUES
  (1, 1, 'QA Officer', '2026-07-15',
   '{"Greeting":"YES","Compliance":"YES","Resolution":"GOOD","Tone":"YES"}',
   '{}',
   0.8571);

-- Ben: improved, no autofail this week
INSERT INTO reviews (scorecard_id, agent_id, reviewer, date, selections, comments, score) VALUES
  (1, 2, 'QA Officer', '2026-07-16',
   '{"Greeting":"YES","Compliance":"YES","Resolution":"GREAT","Tone":"YES"}',
   '{}',
   1.0);

-- Clara: improved tone, still struggling with resolution
INSERT INTO reviews (scorecard_id, agent_id, reviewer, date, selections, comments, score) VALUES
  (1, 3, 'QA Officer', '2026-07-17',
   '{"Greeting":"YES","Compliance":"YES","Resolution":"POOR","Tone":"YES"}',
   '{"Resolution":"Escalated but did not follow up"}',
   0.6667);

-- Alice: second review this week with adjusted score
INSERT INTO reviews (scorecard_id, agent_id, reviewer, date, selections, comments, score, adj_score) VALUES
  (1, 1, 'QA Officer', '2026-07-18',
   '{"Greeting":"YES","Compliance":"YES","Resolution":"GREAT","Tone":"YES"}',
   '{}',
   1.0, 0.95);

-- ── Reports ───────────────────────────────────────────────────
INSERT INTO reports (id, scorecard_id, label, start_date, end_date, accumulation_period) VALUES
  (1, 1, 'Week 1', '2026-07-07', '2026-07-13', NULL),
  (2, 1, 'Week 2', '2026-07-14', '2026-07-20', NULL);