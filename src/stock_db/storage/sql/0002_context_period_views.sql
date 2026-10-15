DROP VIEW IF EXISTS canonical_financial_items;
DROP VIEW IF EXISTS financial_statement_contexts;
DROP TRIGGER IF EXISTS financial_items_delete;
DROP VIEW IF EXISTS financial_items;

CREATE VIEW financial_items AS
SELECT
    fv.ticker,
    CASE
        WHEN instr(fv.period, '|') > 0
            THEN substr(fv.period, 1, instr(fv.period, '|') - 1)
        ELSE fv.period
    END AS period,
    fid.statement,
    fid.item_name,
    fv.value,
    ds.source_code AS source,
    fv.updated_at
FROM financial_values AS fv
JOIN financial_item_defs AS fid
  ON fid.item_id = fv.item_id
JOIN data_sources AS ds
  ON ds.source_id = fv.source_id;

CREATE TRIGGER financial_items_delete
INSTEAD OF DELETE ON financial_items
BEGIN
    DELETE FROM financial_values
    WHERE ticker = OLD.ticker
      AND (
          period = OLD.period
          OR period LIKE OLD.period || '|%'
      )
      AND item_id = (
          SELECT item_id FROM financial_item_defs
          WHERE statement = OLD.statement AND item_name = OLD.item_name
      )
      AND source_id = (
          SELECT source_id FROM data_sources WHERE source_code = OLD.source
      );
END;

CREATE VIEW financial_statement_contexts AS
WITH raw_contexts AS (
    SELECT
        f.ticker,
        f.period,
        'bs' AS statement,
        ds.source_code AS source,
        f.period_start,
        f.period_end,
        f.duration_days,
        f.consolidation_scope,
        f.context_id,
        f.period || '|' || COALESCE(f.period_start, '') || '|' || f.period_end || '|' || f.context_id
            AS context_key
    FROM xbrl_balance_sheet_facts AS f
    JOIN data_sources AS ds
      ON ds.source_id = f.source_id
    WHERE f.consolidation_scope = 'consolidated'
      AND f.dimensions_json = '[]'

    UNION ALL

    SELECT
        f.ticker,
        f.period,
        'pl' AS statement,
        ds.source_code AS source,
        f.period_start,
        f.period_end,
        f.duration_days,
        f.consolidation_scope,
        f.context_id,
        f.period || '|' || COALESCE(f.period_start, '') || '|' || f.period_end || '|' || f.context_id
            AS context_key
    FROM xbrl_profit_loss_facts AS f
    JOIN data_sources AS ds
      ON ds.source_id = f.source_id
    WHERE f.consolidation_scope = 'consolidated'
      AND f.dimensions_json = '[]'

    UNION ALL

    SELECT
        f.ticker,
        f.period,
        'cf' AS statement,
        ds.source_code AS source,
        f.period_start,
        f.period_end,
        f.duration_days,
        f.consolidation_scope,
        f.context_id,
        f.period || '|' || COALESCE(f.period_start, '') || '|' || f.period_end || '|' || f.context_id
            AS context_key
    FROM xbrl_cash_flow_facts AS f
    JOIN data_sources AS ds
      ON ds.source_id = f.source_id
    WHERE f.consolidation_scope = 'consolidated'
      AND f.dimensions_json = '[]'
)
SELECT DISTINCT
    ticker,
    period,
    statement,
    source,
    period_start,
    period_end,
    duration_days,
    consolidation_scope AS reporting_scope,
    context_id,
    context_key
FROM raw_contexts;

CREATE VIEW canonical_financial_items AS
WITH exact_contexts AS (
    SELECT DISTINCT
        ticker,
        source,
        context_key,
        period_start,
        period_end,
        duration_days,
        reporting_scope,
        context_id
    FROM financial_statement_contexts
)
SELECT
    fv.ticker,
    CASE
        WHEN instr(fv.period, '|') > 0
            THEN substr(fv.period, 1, instr(fv.period, '|') - 1)
        ELSE fv.period
    END AS period,
    fid.statement,
    CASE
        WHEN fid.statement = 'pl' AND fid.item_name = 'earnings_per_share' THEN 'eps'
        WHEN fid.statement = 'pl' AND fid.item_name = 'diluted_earnings_per_share' THEN 'diluted_eps'
        ELSE fid.item_name
    END AS item_name,
    fv.value,
    ds.source_code AS source,
    fv.updated_at,
    CASE WHEN ds.source_code = 'edinet_xbrl' THEN ec.period_start END AS period_start,
    CASE WHEN ds.source_code = 'edinet_xbrl' THEN ec.period_end END AS period_end,
    CASE WHEN ds.source_code = 'edinet_xbrl' THEN ec.duration_days END AS duration_days,
    CASE WHEN ds.source_code = 'edinet_xbrl' THEN ec.reporting_scope END AS reporting_scope,
    CASE WHEN ds.source_code = 'edinet_xbrl' THEN ec.context_id END AS context_id
FROM financial_values AS fv
JOIN financial_item_defs AS fid
  ON fid.item_id = fv.item_id
JOIN data_sources AS ds
  ON ds.source_id = fv.source_id
LEFT JOIN exact_contexts AS ec
  ON ec.ticker = fv.ticker
 AND ec.source = ds.source_code
 AND ec.context_key = fv.period
WHERE
       (fid.statement = 'forecast'
        AND fid.item_name IN ('eps_current', 'eps_next')
        AND ds.source_code = 'computed')
    OR (fid.statement = 'forecast'
        AND fid.item_name NOT IN ('eps_current', 'eps_next')
        AND ds.source_code = 'shikiho')
    OR (fid.statement = 'dividend' AND ds.source_code = 'shikiho')
    OR (fid.statement IN ('bs', 'pl', 'cf') AND ds.source_code = 'edinet_xbrl');
