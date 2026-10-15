CREATE VIEW prices AS
WITH ranked AS (
    SELECT
        po.ticker,
        po.date,
        po.close,
        po.volume,
        po.updated_at,
        ds.source_code AS source,
        ROW_NUMBER() OVER (
            PARTITION BY po.ticker, po.date
            ORDER BY
                CASE ds.source_code
                    WHEN 'stooq' THEN 0
                    WHEN 'yahoo_finance_jp' THEN 1
                    ELSE 100
                END,
                po.updated_at DESC,
                ds.source_code
        ) AS source_rank
    FROM price_observations AS po
    JOIN data_sources AS ds
      ON ds.source_id = po.source_id
)
SELECT
    ticker,
    date,
    close,
    volume,
    updated_at,
    source
FROM ranked
WHERE source_rank = 1;

CREATE VIEW stocks AS
SELECT
    se.ticker,
    se.edinet_code,
    se.name,
    se.sector,
    se.market,
    ssc.shares_outstanding,
    ssc.shares_updated_at,
    se.securities_report_url,
    se.yf_suffix,
    se.updated_at
FROM stock_entities AS se
LEFT JOIN stock_share_counts AS ssc
  ON ssc.ticker = se.ticker;

CREATE VIEW financial_items AS
SELECT
    fv.ticker,
    fv.period,
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
        f.context_id
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
        f.context_id
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
        f.context_id
    FROM xbrl_cash_flow_facts AS f
    JOIN data_sources AS ds
      ON ds.source_id = f.source_id
    WHERE f.consolidation_scope = 'consolidated'
      AND f.dimensions_json = '[]'
),
distinct_contexts AS (
    SELECT DISTINCT
        ticker,
        period,
        statement,
        source,
        period_start,
        period_end,
        duration_days,
        consolidation_scope,
        context_id
    FROM raw_contexts
),
ranked AS (
    SELECT
        ticker,
        period,
        statement,
        source,
        period_start,
        period_end,
        duration_days,
        consolidation_scope,
        context_id,
        ROW_NUMBER() OVER (
            PARTITION BY ticker, period, statement, source
            ORDER BY
                CASE
                    WHEN statement IN ('pl', 'cf') THEN COALESCE(duration_days, -1)
                    ELSE -1
                END DESC,
                period_end DESC,
                COALESCE(period_start, '') ASC,
                context_id ASC
        ) AS context_rank
    FROM distinct_contexts
)
SELECT
    ticker,
    period,
    statement,
    source,
    period_start,
    period_end,
    duration_days,
    consolidation_scope AS reporting_scope
FROM ranked
WHERE context_rank = 1;

CREATE VIEW canonical_financial_items AS
SELECT
    fi.ticker,
    fi.period,
    fi.statement,
    CASE
        WHEN fi.statement = 'pl' AND fi.item_name = 'earnings_per_share' THEN 'eps'
        WHEN fi.statement = 'pl' AND fi.item_name = 'diluted_earnings_per_share' THEN 'diluted_eps'
        ELSE fi.item_name
    END AS item_name,
    fi.value,
    fi.source,
    fi.updated_at,
    CASE WHEN fi.source = 'edinet_xbrl' THEN fsc.period_start END AS period_start,
    CASE WHEN fi.source = 'edinet_xbrl' THEN fsc.period_end END AS period_end,
    CASE WHEN fi.source = 'edinet_xbrl' THEN fsc.duration_days END AS duration_days,
    CASE WHEN fi.source = 'edinet_xbrl' THEN fsc.reporting_scope END AS reporting_scope
FROM financial_items AS fi
LEFT JOIN financial_statement_contexts AS fsc
  ON fsc.ticker = fi.ticker
 AND fsc.period = fi.period
 AND fsc.statement = fi.statement
 AND fsc.source = fi.source
WHERE
       (fi.statement = 'forecast'
        AND fi.item_name IN ('eps_current', 'eps_next')
        AND fi.source = 'computed')
    OR (fi.statement = 'forecast'
        AND fi.item_name NOT IN ('eps_current', 'eps_next')
        AND fi.source = 'shikiho')
    OR (fi.statement = 'dividend' AND fi.source = 'shikiho')
    OR (fi.statement IN ('bs', 'pl', 'cf') AND fi.source = 'edinet_xbrl');

CREATE VIEW share_classes AS
SELECT
    scd.ticker,
    sci.period,
    ds.source_code AS source,
    scd.class_key,
    scd.class_name,
    sci.shares,
    scd.is_preferred,
    sk.source_kind,
    sci.updated_at
FROM share_class_issuances AS sci
JOIN share_class_defs AS scd
  ON scd.share_class_id = sci.share_class_id
JOIN data_sources AS ds
  ON ds.source_id = sci.source_id
JOIN source_kinds AS sk
  ON sk.source_kind_id = sci.source_kind_id;

CREATE VIEW preferred_share_terms AS
WITH fact_values AS (
    SELECT
        pstb.text_block_id,
        MAX(CASE WHEN pstf.fact_name = 'preferred_shares'
                 THEN pstf.value END) AS preferred_shares,
        MAX(CASE WHEN pstf.fact_name = 'converted_common_shares'
                 THEN pstf.value END) AS converted_common_shares,
        MAX(CASE WHEN pstf.fact_name = 'conversion_ratio_to_common'
                 THEN pstf.value END) AS conversion_ratio_to_common,
        MAX(CASE WHEN pstf.fact_name = 'issue_price_yen'
                 THEN pstf.value END) AS issue_price_yen,
        MAX(CASE WHEN pstf.fact_name = 'capitalized_amount_yen'
                 THEN pstf.value END) AS capitalized_amount_yen
    FROM preferred_share_text_blocks AS pstb
    LEFT JOIN preferred_share_term_facts AS pstf
      ON pstf.text_block_id = pstb.text_block_id
    GROUP BY pstb.text_block_id
)
SELECT
    scd.ticker,
    pstb.period,
    ds.source_code AS source,
    scd.class_key,
    scd.class_name,
    pstb.terms_text,
    fv.preferred_shares,
    fv.converted_common_shares,
    fv.conversion_ratio_to_common,
    fv.issue_price_yen,
    fv.capitalized_amount_yen,
    pstb.source_concept,
    sk.source_kind,
    pstb.updated_at
FROM preferred_share_text_blocks AS pstb
JOIN share_class_defs AS scd
  ON scd.share_class_id = pstb.share_class_id
JOIN data_sources AS ds
  ON ds.source_id = pstb.source_id
JOIN source_kinds AS sk
  ON sk.source_kind_id = pstb.source_kind_id
JOIN fact_values AS fv
  ON fv.text_block_id = pstb.text_block_id;

CREATE VIEW preferred_share_dilution_terms AS
WITH raw AS (
    SELECT
        pst.*,
        CASE pst.source_kind
            WHEN 'issued_share_change_text' THEN 0
            WHEN 'issued_shares_text' THEN 1
            WHEN 'classes_of_shares_text' THEN 2
            WHEN 'per_share_information_text' THEN 3
            WHEN 'business_results_text' THEN 4
            ELSE 9
        END AS source_priority,
        (pst.preferred_shares IS NOT NULL)
        + (pst.converted_common_shares IS NOT NULL)
        + (pst.conversion_ratio_to_common IS NOT NULL)
        + (pst.issue_price_yen IS NOT NULL)
        + (pst.capitalized_amount_yen IS NOT NULL) AS value_count
    FROM preferred_share_terms AS pst
),
groups AS (
    SELECT DISTINCT ticker, period, source, class_key
    FROM raw
),
selected AS (
    SELECT
        g.ticker,
        g.period,
        g.source,
        g.class_key,
        (
            SELECT r.class_name
            FROM raw AS r
            WHERE r.ticker = g.ticker
              AND r.period = g.period
              AND r.source = g.source
              AND r.class_key = g.class_key
            ORDER BY r.source_priority, r.value_count DESC, r.source_concept, r.terms_text
            LIMIT 1
        ) AS class_name,
        (
            SELECT r.preferred_shares
            FROM raw AS r
            WHERE r.ticker = g.ticker
              AND r.period = g.period
              AND r.source = g.source
              AND r.class_key = g.class_key
              AND r.preferred_shares IS NOT NULL
            ORDER BY r.source_priority, r.value_count DESC, r.source_concept, r.terms_text
            LIMIT 1
        ) AS preferred_shares,
        (
            SELECT r.converted_common_shares
            FROM raw AS r
            WHERE r.ticker = g.ticker
              AND r.period = g.period
              AND r.source = g.source
              AND r.class_key = g.class_key
              AND r.converted_common_shares IS NOT NULL
            ORDER BY r.source_priority, r.value_count DESC, r.source_concept, r.terms_text
            LIMIT 1
        ) AS converted_common_shares,
        (
            SELECT r.conversion_ratio_to_common
            FROM raw AS r
            WHERE r.ticker = g.ticker
              AND r.period = g.period
              AND r.source = g.source
              AND r.class_key = g.class_key
              AND r.conversion_ratio_to_common IS NOT NULL
            ORDER BY r.source_priority, r.value_count DESC, r.source_concept, r.terms_text
            LIMIT 1
        ) AS conversion_ratio_to_common,
        (
            SELECT r.issue_price_yen
            FROM raw AS r
            WHERE r.ticker = g.ticker
              AND r.period = g.period
              AND r.source = g.source
              AND r.class_key = g.class_key
              AND r.issue_price_yen IS NOT NULL
            ORDER BY r.source_priority, r.value_count DESC, r.source_concept, r.terms_text
            LIMIT 1
        ) AS issue_price_yen,
        (
            SELECT r.capitalized_amount_yen
            FROM raw AS r
            WHERE r.ticker = g.ticker
              AND r.period = g.period
              AND r.source = g.source
              AND r.class_key = g.class_key
              AND r.capitalized_amount_yen IS NOT NULL
            ORDER BY r.source_priority, r.value_count DESC, r.source_concept, r.terms_text
            LIMIT 1
        ) AS capitalized_amount_yen,
        (
            SELECT r.source_concept
            FROM raw AS r
            WHERE r.ticker = g.ticker
              AND r.period = g.period
              AND r.source = g.source
              AND r.class_key = g.class_key
            ORDER BY r.source_priority, r.value_count DESC, r.source_concept, r.terms_text
            LIMIT 1
        ) AS source_concept,
        (
            SELECT r.source_kind
            FROM raw AS r
            WHERE r.ticker = g.ticker
              AND r.period = g.period
              AND r.source = g.source
              AND r.class_key = g.class_key
            ORDER BY r.source_priority, r.value_count DESC, r.source_concept, r.terms_text
            LIMIT 1
        ) AS source_kind,
        (
            SELECT r.updated_at
            FROM raw AS r
            WHERE r.ticker = g.ticker
              AND r.period = g.period
              AND r.source = g.source
              AND r.class_key = g.class_key
            ORDER BY r.source_priority, r.value_count DESC, r.source_concept, r.terms_text
            LIMIT 1
        ) AS updated_at
    FROM groups AS g
)
SELECT
    ticker,
    period,
    source,
    class_key,
    class_name,
    preferred_shares,
    CASE
        WHEN converted_common_shares IS NULL
         AND preferred_shares IS NOT NULL
         AND conversion_ratio_to_common IS NOT NULL
        THEN preferred_shares * conversion_ratio_to_common
        ELSE converted_common_shares
    END AS converted_common_shares,
    CASE
        WHEN conversion_ratio_to_common IS NULL
         AND preferred_shares IS NOT NULL
         AND preferred_shares != 0
         AND converted_common_shares IS NOT NULL
        THEN converted_common_shares / preferred_shares
        ELSE conversion_ratio_to_common
    END AS conversion_ratio_to_common,
    issue_price_yen,
    capitalized_amount_yen,
    source_concept,
    source_kind,
    updated_at
FROM selected;

CREATE VIEW potential_equity_instruments AS
SELECT
    pet.ticker,
    pet.period,
    ds.source_code AS source,
    pet.instrument_key,
    pet.instrument_type,
    pet.instrument_name,
    pet.terms_text,
    pet.period_end_common_shares,
    pet.exercise_price_yen,
    pet.conversion_price_yen,
    pet.face_value_yen,
    pet.source_concept,
    sk.source_kind,
    pet.updated_at
FROM potential_equity_instruments_raw AS pet
JOIN data_sources AS ds
  ON ds.source_id = pet.source_id
JOIN source_kinds AS sk
  ON sk.source_kind_id = pet.source_kind_id;

CREATE VIEW potential_equity_status AS
SELECT
    pes.ticker,
    pes.period,
    ds.source_code AS source,
    pes.status_key,
    pes.has_potential_equity,
    pes.terms_text,
    pes.source_concept,
    sk.source_kind,
    pes.updated_at
FROM potential_equity_status_raw AS pes
JOIN data_sources AS ds
  ON ds.source_id = pes.source_id
JOIN source_kinds AS sk
  ON sk.source_kind_id = pes.source_kind_id;

CREATE VIEW diluted_eps_adjustments AS
SELECT
    dea.ticker,
    dea.period,
    ds.source_code AS source,
    dea.adjustment_key,
    dea.adjustment_type,
    dea.adjustment_name,
    dea.terms_text,
    dea.weighted_average_common_shares,
    dea.source_concept,
    sk.source_kind,
    dea.updated_at
FROM diluted_eps_adjustments_raw AS dea
JOIN data_sources AS ds
  ON ds.source_id = dea.source_id
JOIN source_kinds AS sk
  ON sk.source_kind_id = dea.source_kind_id;

CREATE VIEW source_refresh_log AS
SELECT
    ds.source_code AS source,
    src.target_date,
    src.checked_at
FROM source_refresh_checks AS src
JOIN data_sources AS ds
  ON ds.source_id = src.source_id;

CREATE VIEW sec_reports AS
SELECT
    srr.ticker,
    srr.fiscal_year,
    srr.doc_id,
    dt.doc_type,
    srr.xbrl_path,
    ds.source_code AS source,
    srr.updated_at
FROM sec_report_records AS srr
JOIN doc_types AS dt
  ON dt.doc_type_id = srr.doc_type_id
JOIN data_sources AS ds
  ON ds.source_id = srr.source_id;

CREATE TRIGGER financial_items_delete
INSTEAD OF DELETE ON financial_items
BEGIN
    DELETE FROM financial_values
    WHERE ticker = OLD.ticker
      AND period = OLD.period
      AND item_id = (
          SELECT item_id FROM financial_item_defs
          WHERE statement = OLD.statement AND item_name = OLD.item_name
      )
      AND source_id = (
          SELECT source_id FROM data_sources WHERE source_code = OLD.source
      );
END;

CREATE TRIGGER share_classes_delete
INSTEAD OF DELETE ON share_classes
BEGIN
    DELETE FROM share_class_issuances
    WHERE period = OLD.period
      AND source_id = (
          SELECT source_id FROM data_sources WHERE source_code = OLD.source
      )
      AND share_class_id = (
          SELECT share_class_id FROM share_class_defs
          WHERE ticker = OLD.ticker AND class_key = OLD.class_key
      );
END;

CREATE TRIGGER preferred_share_terms_delete
INSTEAD OF DELETE ON preferred_share_terms
BEGIN
    DELETE FROM preferred_share_text_blocks
    WHERE period = OLD.period
      AND source_id = (
          SELECT source_id FROM data_sources WHERE source_code = OLD.source
      )
      AND share_class_id = (
          SELECT share_class_id FROM share_class_defs
          WHERE ticker = OLD.ticker AND class_key = OLD.class_key
      )
      AND source_concept = OLD.source_concept;
END;

CREATE TRIGGER preferred_share_dilution_terms_delete
INSTEAD OF DELETE ON preferred_share_dilution_terms
BEGIN
    DELETE FROM preferred_share_text_blocks
    WHERE period = OLD.period
      AND source_id = (
          SELECT source_id FROM data_sources WHERE source_code = OLD.source
      )
      AND share_class_id = (
          SELECT share_class_id FROM share_class_defs
          WHERE ticker = OLD.ticker AND class_key = OLD.class_key
      );
END;