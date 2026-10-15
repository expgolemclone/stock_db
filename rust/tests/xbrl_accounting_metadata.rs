use std::fs;

use edinet_xbrl::{db, schema};
use rusqlite::{Connection, params};

#[test]
fn reported_eps_preserves_period_unit_and_declared_precision_end_to_end() {
    let temp = tempfile::tempdir().unwrap();
    let xbrl_path = temp.path().join("sample.xbrl");
    fs::write(
        &xbrl_path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<xbrli:xbrl
    xmlns:xbrli="http://www.xbrl.org/2003/instance"
    xmlns:iso4217="http://www.xbrl.org/2003/iso4217"
    xmlns:jppfs_cor="http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2025-11-01/jppfs_cor">
  <xbrli:context id="CurrentYearInstant">
    <xbrli:entity><xbrli:identifier scheme="test">E00001</xbrli:identifier></xbrli:entity>
    <xbrli:period><xbrli:instant>2025-03-31</xbrli:instant></xbrli:period>
  </xbrli:context>
  <xbrli:context id="CurrentYearDuration">
    <xbrli:entity><xbrli:identifier scheme="test">E00001</xbrli:identifier></xbrli:entity>
    <xbrli:period>
      <xbrli:startDate>2024-04-01</xbrli:startDate>
      <xbrli:endDate>2025-03-31</xbrli:endDate>
    </xbrli:period>
  </xbrli:context>
  <xbrli:unit id="JPY"><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unit>
  <xbrli:unit id="JPYPerShare">
    <xbrli:divide>
      <xbrli:unitNumerator><xbrli:measure>iso4217:JPY</xbrli:measure></xbrli:unitNumerator>
      <xbrli:unitDenominator><xbrli:measure>xbrli:shares</xbrli:measure></xbrli:unitDenominator>
    </xbrli:divide>
  </xbrli:unit>
  <jppfs_cor:Assets contextRef="CurrentYearInstant" unitRef="JPY" decimals="-3">1000000</jppfs_cor:Assets>
  <jppfs_cor:BasicEarningsLossPerShare contextRef="CurrentYearDuration" unitRef="JPYPerShare" decimals="2">123.45</jppfs_cor:BasicEarningsLossPerShare>
  <jppfs_cor:DilutedEarningsLossPerShare contextRef="CurrentYearDuration" unitRef="JPYPerShare" decimals="2">120.00</jppfs_cor:DilutedEarningsLossPerShare>
</xbrli:xbrl>
"#,
    )
    .unwrap();

    let db_path = temp.path().join("stocks.db");
    let conn = Connection::open(&db_path).unwrap();
    schema::init_db(&conn).unwrap();
    conn.execute(
        "INSERT INTO stock_entities (ticker, edinet_code, name) VALUES ('1234', 'E00001', 'test')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO data_sources (source_code) VALUES ('edinet_api')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO doc_types (doc_type) VALUES ('annual')",
        [],
    )
    .unwrap();
    let source_id: i64 = conn
        .query_row(
            "SELECT source_id FROM data_sources WHERE source_code = 'edinet_api'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let doc_type_id: i64 = conn
        .query_row(
            "SELECT doc_type_id FROM doc_types WHERE doc_type = 'annual'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    conn.execute(
        r#"
        INSERT INTO sec_report_records
            (ticker, fiscal_year, doc_id, doc_type_id, xbrl_path, source_id, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            "1234",
            "2025",
            "SAMPLE1",
            doc_type_id,
            xbrl_path.to_str().unwrap(),
            source_id,
            "2026-08-09T00:00:00Z",
        ],
    )
    .unwrap();
    drop(conn);

    let summary = db::parse_xbrl_financials_to_db(
        db_path.to_str().unwrap(),
        Some("1234"),
        None,
        false,
        false,
        None,
        1,
    )
    .unwrap();
    assert_eq!(summary.ok, 1);
    assert_eq!(summary.errors, 0);

    let conn = Connection::open(&db_path).unwrap();
    let reported: Vec<(String, f64, String, Option<String>, String, Option<i64>, String)> = conn
        .prepare(
            r#"
            SELECT item_name, value, source, period_start, period_end,
                   duration_days, reporting_scope
            FROM canonical_financial_items
            WHERE ticker = '1234' AND period = '2025-03' AND statement = 'pl'
              AND item_name IN ('eps', 'diluted_eps')
            ORDER BY item_name
            "#,
        )
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        reported,
        vec![
            (
                "diluted_eps".to_string(),
                120.0,
                "edinet_xbrl".to_string(),
                Some("2024-04-01".to_string()),
                "2025-03-31".to_string(),
                Some(365),
                "consolidated".to_string(),
            ),
            (
                "eps".to_string(),
                123.45,
                "edinet_xbrl".to_string(),
                Some("2024-04-01".to_string()),
                "2025-03-31".to_string(),
                Some(365),
                "consolidated".to_string(),
            ),
        ]
    );

    let raw: (Option<String>, String, Option<i64>, String, String, f64) = conn
        .query_row(
            r#"
            SELECT period_start, period_end, duration_days, unit, decimals, value
            FROM xbrl_profit_loss_facts
            WHERE ticker = '1234'
              AND concept_name = 'BasicEarningsLossPerShare'
            "#,
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(raw.0.as_deref(), Some("2024-04-01"));
    assert_eq!(raw.1, "2025-03-31");
    assert_eq!(raw.2, Some(365));
    assert_eq!(raw.3, "JPY/share");
    assert_eq!(raw.4, "2");
    assert_eq!(raw.5, 123.45);
}
