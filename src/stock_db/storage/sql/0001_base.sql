CREATE TABLE IF NOT EXISTS data_sources (
    source_id   INTEGER PRIMARY KEY,
    source_code TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS stock_entities (
    ticker                TEXT PRIMARY KEY,
    edinet_code           TEXT,
    name                  TEXT NOT NULL DEFAULT '',
    sector                TEXT NOT NULL DEFAULT '',
    market                TEXT NOT NULL DEFAULT '',
    securities_report_url TEXT,
    yf_suffix             TEXT,
    updated_at            TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_stock_entities_edinet_code
    ON stock_entities (edinet_code) WHERE edinet_code IS NOT NULL;

CREATE TABLE IF NOT EXISTS stock_share_counts (
    ticker              TEXT PRIMARY KEY,
    shares_outstanding  INTEGER NOT NULL CHECK (shares_outstanding > 0),
    shares_updated_at   TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    FOREIGN KEY (ticker) REFERENCES stock_entities (ticker)
);

CREATE TABLE IF NOT EXISTS price_observations (
    ticker        TEXT    NOT NULL,
    date          TEXT    NOT NULL
        CHECK (date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    source_id     INTEGER NOT NULL,
    close         REAL    NOT NULL CHECK (close > 0),
    volume        INTEGER CHECK (volume IS NULL OR volume >= 0),
    updated_at    TEXT    NOT NULL,
    PRIMARY KEY (ticker, date, source_id),
    FOREIGN KEY (ticker) REFERENCES stock_entities (ticker),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id)
);

CREATE INDEX IF NOT EXISTS idx_price_observations_latest
    ON price_observations (ticker, date DESC);

CREATE TABLE IF NOT EXISTS unresolved_price_observations (
    ticker      TEXT    NOT NULL,
    date        TEXT    NOT NULL
        CHECK (date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    source_id   INTEGER NOT NULL,
    close       REAL    NOT NULL CHECK (close > 0),
    volume      INTEGER CHECK (volume IS NULL OR volume >= 0),
    reason      TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL,
    PRIMARY KEY (ticker, date, source_id),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id)
);

CREATE TABLE IF NOT EXISTS source_refresh_checks (
    source_id   INTEGER PRIMARY KEY,
    target_date TEXT NOT NULL
        CHECK (target_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    checked_at  TEXT NOT NULL,
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id)
);

CREATE TABLE IF NOT EXISTS financial_item_defs (
    item_id    INTEGER PRIMARY KEY,
    statement  TEXT NOT NULL,
    item_name  TEXT NOT NULL,
    UNIQUE (statement, item_name)
);

CREATE TABLE IF NOT EXISTS financial_values (
    ticker     TEXT    NOT NULL,
    period     TEXT    NOT NULL,
    item_id    INTEGER NOT NULL,
    source_id  INTEGER NOT NULL,
    value      REAL,
    updated_at TEXT    NOT NULL,
    PRIMARY KEY (ticker, period, item_id, source_id),
    FOREIGN KEY (ticker) REFERENCES stock_entities (ticker),
    FOREIGN KEY (item_id) REFERENCES financial_item_defs (item_id),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id)
);

CREATE INDEX IF NOT EXISTS idx_financial_values_ticker
    ON financial_values (ticker);

CREATE INDEX IF NOT EXISTS idx_financial_values_source
    ON financial_values (source_id);

CREATE TABLE IF NOT EXISTS xbrl_balance_sheet_facts (
    ticker               TEXT    NOT NULL,
    doc_id               TEXT    NOT NULL,
    period               TEXT    NOT NULL,
    period_start         TEXT,
    period_end           TEXT    NOT NULL,
    duration_days        INTEGER CHECK (duration_days IS NULL OR duration_days >= 0),
    source_id            INTEGER NOT NULL,
    consolidation_scope  TEXT    NOT NULL,
    context_id           TEXT    NOT NULL,
    dimensions_json      TEXT    NOT NULL DEFAULT '[]',
    concept_namespace    TEXT    NOT NULL,
    concept_name         TEXT    NOT NULL,
    canonical_statement  TEXT,
    canonical_item_name  TEXT,
    taxonomy_status      TEXT    NOT NULL DEFAULT 'unknown',
    role_uri             TEXT    NOT NULL DEFAULT '',
    unit                 TEXT    NOT NULL,
    unit_ref             TEXT    NOT NULL,
    decimals             TEXT    NOT NULL DEFAULT '',
    value                REAL,
    updated_at           TEXT    NOT NULL,
    PRIMARY KEY (
        ticker, doc_id, period, source_id, consolidation_scope,
        context_id, concept_namespace, concept_name, role_uri
    ),
    FOREIGN KEY (ticker) REFERENCES stock_entities (ticker),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id)
);

CREATE INDEX IF NOT EXISTS idx_xbrl_balance_sheet_facts_ticker
    ON xbrl_balance_sheet_facts (ticker);

CREATE INDEX IF NOT EXISTS idx_xbrl_balance_sheet_facts_concept
    ON xbrl_balance_sheet_facts (concept_name);

CREATE INDEX IF NOT EXISTS idx_xbrl_balance_sheet_facts_role
    ON xbrl_balance_sheet_facts (role_uri);

CREATE TABLE IF NOT EXISTS xbrl_statement_presentation_edges (
    ticker              TEXT    NOT NULL,
    doc_id              TEXT    NOT NULL,
    statement           TEXT    NOT NULL,
    source_id           INTEGER NOT NULL,
    role_uri            TEXT    NOT NULL DEFAULT '',
    parent_namespace    TEXT    NOT NULL,
    parent_name         TEXT    NOT NULL,
    parent_label        TEXT    NOT NULL DEFAULT '',
    child_namespace     TEXT    NOT NULL,
    child_name          TEXT    NOT NULL,
    child_order         REAL    NOT NULL DEFAULT 0,
    child_label         TEXT    NOT NULL DEFAULT '',
    updated_at          TEXT    NOT NULL,
    PRIMARY KEY (
        ticker, doc_id, statement, source_id, role_uri,
        parent_namespace, parent_name, child_namespace, child_name
    ),
    FOREIGN KEY (ticker) REFERENCES stock_entities (ticker),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id)
);

CREATE INDEX IF NOT EXISTS idx_xbrl_statement_presentation_edges_ticker
    ON xbrl_statement_presentation_edges (ticker);

CREATE INDEX IF NOT EXISTS idx_xbrl_statement_presentation_edges_child
    ON xbrl_statement_presentation_edges (child_name);

CREATE TABLE IF NOT EXISTS xbrl_profit_loss_facts (
    ticker               TEXT    NOT NULL,
    doc_id               TEXT    NOT NULL,
    period               TEXT    NOT NULL,
    period_start         TEXT,
    period_end           TEXT    NOT NULL,
    duration_days        INTEGER CHECK (duration_days IS NULL OR duration_days >= 0),
    source_id            INTEGER NOT NULL,
    consolidation_scope  TEXT    NOT NULL,
    context_id           TEXT    NOT NULL,
    dimensions_json      TEXT    NOT NULL DEFAULT '[]',
    concept_namespace    TEXT    NOT NULL,
    concept_name         TEXT    NOT NULL,
    canonical_statement  TEXT,
    canonical_item_name  TEXT,
    taxonomy_status      TEXT    NOT NULL DEFAULT 'unknown',
    role_uri             TEXT    NOT NULL DEFAULT '',
    unit                 TEXT    NOT NULL,
    unit_ref             TEXT    NOT NULL,
    decimals             TEXT    NOT NULL DEFAULT '',
    value                REAL,
    updated_at           TEXT    NOT NULL,
    PRIMARY KEY (
        ticker, doc_id, period, source_id, consolidation_scope,
        context_id, concept_namespace, concept_name, role_uri
    ),
    FOREIGN KEY (ticker) REFERENCES stock_entities (ticker),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id)
);

CREATE INDEX IF NOT EXISTS idx_xbrl_profit_loss_facts_ticker
    ON xbrl_profit_loss_facts (ticker);

CREATE INDEX IF NOT EXISTS idx_xbrl_profit_loss_facts_concept
    ON xbrl_profit_loss_facts (concept_name);

CREATE INDEX IF NOT EXISTS idx_xbrl_profit_loss_facts_role
    ON xbrl_profit_loss_facts (role_uri);

CREATE TABLE IF NOT EXISTS xbrl_cash_flow_facts (
    ticker               TEXT    NOT NULL,
    doc_id               TEXT    NOT NULL,
    period               TEXT    NOT NULL,
    period_start         TEXT,
    period_end           TEXT    NOT NULL,
    duration_days        INTEGER CHECK (duration_days IS NULL OR duration_days >= 0),
    source_id            INTEGER NOT NULL,
    consolidation_scope  TEXT    NOT NULL,
    context_id           TEXT    NOT NULL,
    dimensions_json      TEXT    NOT NULL DEFAULT '[]',
    concept_namespace    TEXT    NOT NULL,
    concept_name         TEXT    NOT NULL,
    canonical_statement  TEXT,
    canonical_item_name  TEXT,
    taxonomy_status      TEXT    NOT NULL DEFAULT 'unknown',
    role_uri             TEXT    NOT NULL DEFAULT '',
    unit                 TEXT    NOT NULL,
    unit_ref             TEXT    NOT NULL,
    decimals             TEXT    NOT NULL DEFAULT '',
    value                REAL,
    updated_at           TEXT    NOT NULL,
    PRIMARY KEY (
        ticker, doc_id, period, source_id, consolidation_scope,
        context_id, concept_namespace, concept_name, role_uri
    ),
    FOREIGN KEY (ticker) REFERENCES stock_entities (ticker),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id)
);

CREATE INDEX IF NOT EXISTS idx_xbrl_cash_flow_facts_ticker
    ON xbrl_cash_flow_facts (ticker);

CREATE INDEX IF NOT EXISTS idx_xbrl_cash_flow_facts_concept
    ON xbrl_cash_flow_facts (concept_name);

CREATE INDEX IF NOT EXISTS idx_xbrl_cash_flow_facts_role
    ON xbrl_cash_flow_facts (role_uri);

CREATE TABLE IF NOT EXISTS source_kinds (
    source_kind_id INTEGER PRIMARY KEY,
    source_kind    TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS share_class_defs (
    share_class_id INTEGER PRIMARY KEY,
    ticker         TEXT    NOT NULL,
    class_key      TEXT    NOT NULL,
    class_name     TEXT    NOT NULL,
    is_preferred   INTEGER NOT NULL CHECK (is_preferred IN (0, 1)),
    updated_at     TEXT    NOT NULL,
    UNIQUE (ticker, class_key),
    FOREIGN KEY (ticker) REFERENCES stock_entities (ticker)
);

CREATE INDEX IF NOT EXISTS idx_share_class_defs_preferred
    ON share_class_defs (is_preferred);

CREATE TABLE IF NOT EXISTS share_class_issuances (
    period          TEXT    NOT NULL,
    source_id       INTEGER NOT NULL,
    share_class_id  INTEGER NOT NULL,
    shares          REAL,
    source_kind_id  INTEGER NOT NULL,
    updated_at      TEXT    NOT NULL,
    PRIMARY KEY (period, source_id, share_class_id),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id),
    FOREIGN KEY (share_class_id) REFERENCES share_class_defs (share_class_id),
    FOREIGN KEY (source_kind_id) REFERENCES source_kinds (source_kind_id)
);

CREATE INDEX IF NOT EXISTS idx_share_class_issuances_period
    ON share_class_issuances (period);

CREATE TABLE IF NOT EXISTS preferred_share_text_blocks (
    text_block_id   INTEGER PRIMARY KEY,
    period          TEXT    NOT NULL,
    source_id       INTEGER NOT NULL,
    share_class_id  INTEGER NOT NULL,
    terms_text      TEXT    NOT NULL,
    source_concept  TEXT    NOT NULL,
    source_kind_id  INTEGER NOT NULL,
    updated_at      TEXT    NOT NULL,
    UNIQUE (period, source_id, share_class_id, source_concept),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id),
    FOREIGN KEY (share_class_id) REFERENCES share_class_defs (share_class_id),
    FOREIGN KEY (source_kind_id) REFERENCES source_kinds (source_kind_id)
);

CREATE TABLE IF NOT EXISTS preferred_share_term_facts (
    text_block_id INTEGER NOT NULL,
    fact_name     TEXT    NOT NULL,
    value         REAL    NOT NULL,
    PRIMARY KEY (text_block_id, fact_name),
    FOREIGN KEY (text_block_id) REFERENCES preferred_share_text_blocks (text_block_id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS potential_equity_instruments_raw (
    instrument_id            INTEGER PRIMARY KEY,
    ticker                   TEXT    NOT NULL,
    period                   TEXT    NOT NULL,
    source_id                INTEGER NOT NULL,
    instrument_key           TEXT    NOT NULL,
    instrument_type          TEXT    NOT NULL,
    instrument_name          TEXT    NOT NULL,
    terms_text               TEXT    NOT NULL,
    period_end_common_shares REAL,
    exercise_price_yen       REAL,
    conversion_price_yen     REAL,
    face_value_yen           REAL,
    source_concept           TEXT    NOT NULL,
    source_kind_id           INTEGER NOT NULL,
    updated_at               TEXT    NOT NULL,
    UNIQUE (ticker, period, source_id, instrument_key, source_concept, terms_text),
    FOREIGN KEY (ticker) REFERENCES stock_entities (ticker),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id),
    FOREIGN KEY (source_kind_id) REFERENCES source_kinds (source_kind_id)
);

CREATE INDEX IF NOT EXISTS idx_potential_equity_instruments_ticker
    ON potential_equity_instruments_raw (ticker);

CREATE TABLE IF NOT EXISTS potential_equity_status_raw (
    status_id            INTEGER PRIMARY KEY,
    ticker               TEXT    NOT NULL,
    period               TEXT    NOT NULL,
    source_id            INTEGER NOT NULL,
    status_key           TEXT    NOT NULL,
    has_potential_equity INTEGER NOT NULL CHECK (has_potential_equity IN (0, 1)),
    terms_text           TEXT    NOT NULL,
    source_concept       TEXT    NOT NULL,
    source_kind_id       INTEGER NOT NULL,
    updated_at           TEXT    NOT NULL,
    UNIQUE (ticker, period, source_id, status_key, source_concept, terms_text),
    FOREIGN KEY (ticker) REFERENCES stock_entities (ticker),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id),
    FOREIGN KEY (source_kind_id) REFERENCES source_kinds (source_kind_id)
);

CREATE INDEX IF NOT EXISTS idx_potential_equity_status_ticker
    ON potential_equity_status_raw (ticker);

CREATE TABLE IF NOT EXISTS diluted_eps_adjustments_raw (
    adjustment_id                  INTEGER PRIMARY KEY,
    ticker                         TEXT    NOT NULL,
    period                         TEXT    NOT NULL,
    source_id                      INTEGER NOT NULL,
    adjustment_key                 TEXT    NOT NULL,
    adjustment_type                TEXT    NOT NULL,
    adjustment_name                TEXT    NOT NULL,
    terms_text                     TEXT    NOT NULL,
    weighted_average_common_shares REAL,
    source_concept                 TEXT    NOT NULL,
    source_kind_id                 INTEGER NOT NULL,
    updated_at                     TEXT    NOT NULL,
    UNIQUE (ticker, period, source_id, adjustment_key, source_concept, terms_text),
    FOREIGN KEY (ticker) REFERENCES stock_entities (ticker),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id),
    FOREIGN KEY (source_kind_id) REFERENCES source_kinds (source_kind_id)
);

CREATE INDEX IF NOT EXISTS idx_diluted_eps_adjustments_ticker
    ON diluted_eps_adjustments_raw (ticker);

CREATE TABLE IF NOT EXISTS doc_types (
    doc_type_id INTEGER PRIMARY KEY,
    doc_type    TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS sec_report_records (
    ticker       TEXT    NOT NULL,
    fiscal_year  TEXT    NOT NULL,
    doc_id       TEXT    NOT NULL,
    doc_type_id  INTEGER NOT NULL,
    xbrl_path    TEXT,
    source_id    INTEGER NOT NULL,
    updated_at   TEXT    NOT NULL,
    PRIMARY KEY (ticker, doc_id),
    UNIQUE (doc_id),
    FOREIGN KEY (ticker) REFERENCES stock_entities (ticker),
    FOREIGN KEY (doc_type_id) REFERENCES doc_types (doc_type_id),
    FOREIGN KEY (source_id) REFERENCES data_sources (source_id)
);

CREATE INDEX IF NOT EXISTS idx_sec_report_records_ticker
    ON sec_report_records (ticker);

CREATE INDEX IF NOT EXISTS idx_sec_report_records_doc_id
    ON sec_report_records (doc_id);