//! `edinet-xbrl` CLI entry point.
//!
//! Phased migration from the Python layer. Each phase attaches more
//! subcommands here until the Python layer is removed in phase 7.

use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use serde::Serialize;

use edinet_xbrl::{config, db, downstream, edinet, paths, shikiho};

#[derive(Parser)]
#[command(
    name = "edinet-xbrl",
    version,
    about = "EDINET XBRL financial data platform"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Audit XBRL taxonomy registry coverage against raw DB facts.
    AuditXbrlTaxonomy(AuditArgs),
    /// Download a single EDINET XBRL ZIP by docID and extract it.
    DownloadXbrl(DownloadXbrlArgs),
    /// Discover and download historical 有価証券報告書 via EDINET API v2.
    ScrapeHistorical(ScrapeHistoricalArgs),
    /// Sync net income forecasts from japan_company_handbook.
    SyncShikihoForecasts(ShikihoArgs),
    /// Sync dividend data from japan_company_handbook.
    SyncShikihoDividends(ShikihoDividendsArgs),
    /// Compute EPS (source=computed) from net income / shares outstanding.
    ComputeEps(DbArgs),
    /// Initialize the stocks.db schema (idempotent).
    InitDb(DbArgs),
    /// Parse selected EDINET XBRL artifacts from SQLite and replace DB rows.
    ParseXbrlFinancials(ParseXbrlFinancialsArgs),
    /// Inspect a ticker's stored financial items.
    InspectStockDb(InspectArgs),
    /// Emit all stock tickers as JSON for downstream repos.
    DownstreamAllTickers(DbArgs),
    /// Emit stock names as JSON for downstream repos.
    DownstreamStockNames(DbArgs),
    /// Emit stock price metadata as JSON for downstream repos.
    DownstreamStockPriceMetadata(DbArgs),
    /// Emit screening-ready tickers as JSON for downstream repos.
    DownstreamScreeningTickers(DownstreamScreeningTickersArgs),
    /// Emit screening read-model rows as JSON for downstream repos.
    DownstreamScreeningStocks(DownstreamScreeningStocksArgs),
    /// Emit market caps as JSON for downstream repos. Tickers are read from stdin.
    DownstreamStockMarketCaps(DownstreamMarketCapsArgs),
    /// Emit latest valid XBRL artifact metadata as JSON. Tickers are read from stdin.
    DownstreamLatestXbrlArtifacts(DownstreamTickersArgs),
    /// Emit balance-sheet presentation-tree histories as JSON. Tickers are read from stdin.
    DownstreamBalanceSheetHistories(DownstreamBalanceSheetHistoriesArgs),
}

#[derive(clap::Args)]
struct DbArgs {
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
}

#[derive(clap::Args)]
struct AuditArgs {
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
    #[arg(long, default_value_t = 1)]
    min_count: usize,
    #[arg(long, value_enum, default_value_t = AuditFormat::Tsv)]
    format: AuditFormat,
    #[arg(long, value_name = "PATH")]
    output: Option<PathBuf>,
    #[arg(long)]
    cf_suffix_collisions: bool,
}

#[derive(clap::ValueEnum, Clone, Copy, PartialEq)]
enum AuditFormat {
    Tsv,
    Json,
}

#[derive(clap::Args)]
struct DownloadXbrlArgs {
    #[arg(long)]
    doc_id: String,
    #[arg(long)]
    ticker: Option<String>,
    #[arg(long, value_name = "PATH")]
    dest_dir: Option<PathBuf>,
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
}

#[derive(clap::Args)]
struct ScrapeHistoricalArgs {
    #[arg(long)]
    from_date: Option<String>,
    #[arg(long)]
    to_date: Option<String>,
    #[arg(long, default_value_t = 10)]
    years: u32,
    #[arg(long)]
    ticker: Option<String>,
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
    #[arg(long)]
    force: bool,
    #[arg(long)]
    interval: Option<f64>,
}

#[derive(clap::Args)]
struct ShikihoArgs {
    #[arg(long, value_name = "PATH")]
    shikiho_db: Option<PathBuf>,
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
}

#[derive(clap::Args)]
struct ShikihoDividendsArgs {
    #[arg(long, value_name = "PATH")]
    shikiho_db: Option<PathBuf>,
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
    #[arg(long, value_name = "PATH")]
    dividend_overrides: Option<PathBuf>,
}

#[derive(clap::Args)]
struct ParseXbrlFinancialsArgs {
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
    #[arg(long)]
    ticker: Option<String>,
    #[arg(long)]
    from_ticker: Option<String>,
    #[arg(long, help = "Disable --skip-existing")]
    force: bool,
    #[arg(long)]
    emit_progress: bool,
    #[arg(long, default_value_t = 25)]
    parse_batch_size: usize,
}

#[derive(clap::Args)]
struct InspectArgs {
    /// Ticker to inspect.
    ticker: String,
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
    #[arg(long, default_value_t = 1)]
    limit: usize,
}

#[derive(clap::Args)]
struct DownstreamTickersArgs {
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
}

#[derive(clap::Args)]
struct DownstreamMarketCapsArgs {
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
    #[arg(long, default_value_t = 7)]
    max_age_days: i64,
}

#[derive(clap::Args)]
struct DownstreamScreeningTickersArgs {
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
    #[arg(long)]
    limit: Option<usize>,
}

#[derive(clap::Args)]
struct DownstreamScreeningStocksArgs {
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
    #[arg(long, default_value_t = 10)]
    fcf_periods: usize,
    #[arg(long, default_value_t = 6)]
    pl_periods: usize,
    #[arg(long, default_value_t = 10)]
    payout_periods: usize,
}

#[derive(clap::Args)]
struct DownstreamBalanceSheetHistoriesArgs {
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
    #[arg(long, default_value_t = 10)]
    n_periods: usize,
    #[arg(long, default_value = "edinet_xbrl")]
    source: String,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::AuditXbrlTaxonomy(args) => run_audit_xbrl_taxonomy(args),
        Command::DownloadXbrl(args) => run_download_xbrl(args),
        Command::ScrapeHistorical(args) => run_scrape_historical(args),
        Command::SyncShikihoForecasts(args) => run_sync_shikiho_forecasts(args),
        Command::SyncShikihoDividends(args) => run_sync_shikiho_dividends(args),
        Command::ComputeEps(args) => run_compute_eps(args),
        Command::InitDb(args) => run_init_db(args),
        Command::ParseXbrlFinancials(args) => run_parse_xbrl_financials(args),
        Command::InspectStockDb(args) => run_inspect_stock_db(args),
        Command::DownstreamAllTickers(args) => emit_json(downstream::get_all_tickers(args.db_path)),
        Command::DownstreamStockNames(args) => emit_json(downstream::get_stock_names(args.db_path)),
        Command::DownstreamStockPriceMetadata(args) => {
            emit_json(downstream::get_stock_price_metadata(args.db_path))
        }
        Command::DownstreamScreeningTickers(args) => {
            emit_json(downstream::get_screening_tickers(args.db_path, args.limit))
        }
        Command::DownstreamScreeningStocks(args) => run_downstream_screening_stocks(args),
        Command::DownstreamStockMarketCaps(args) => run_downstream_stock_market_caps(args),
        Command::DownstreamLatestXbrlArtifacts(args) => run_downstream_latest_xbrl_artifacts(args),
        Command::DownstreamBalanceSheetHistories(args) => {
            run_downstream_balance_sheet_histories(args)
        }
    }
}

fn emit_json<T: Serialize>(result: Result<T, String>) -> ExitCode {
    match result.and_then(|value| downstream::write_json_stdout(&value)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(2)
        }
    }
}

fn read_downstream_tickers() -> Result<Vec<String>, ExitCode> {
    downstream::read_tickers_from_stdin().map_err(|err| {
        eprintln!("{err}");
        ExitCode::from(2)
    })
}

fn run_downstream_screening_stocks(args: DownstreamScreeningStocksArgs) -> ExitCode {
    let tickers = match read_downstream_tickers() {
        Ok(values) => values,
        Err(code) => return code,
    };
    let ticker_refs = if tickers.is_empty() {
        None
    } else {
        Some(tickers.as_slice())
    };
    emit_json(downstream::load_screening_stocks(
        args.db_path,
        ticker_refs,
        args.fcf_periods,
        args.pl_periods,
        args.payout_periods,
    ))
}

fn run_downstream_stock_market_caps(args: DownstreamMarketCapsArgs) -> ExitCode {
    let tickers = match read_downstream_tickers() {
        Ok(values) => values,
        Err(code) => return code,
    };
    emit_json(downstream::get_stock_market_caps(
        args.db_path,
        &tickers,
        args.max_age_days,
    ))
}

fn run_downstream_latest_xbrl_artifacts(args: DownstreamTickersArgs) -> ExitCode {
    let tickers = match read_downstream_tickers() {
        Ok(values) => values,
        Err(code) => return code,
    };
    emit_json(downstream::get_latest_xbrl_artifacts(
        args.db_path,
        &tickers,
    ))
}

fn run_downstream_balance_sheet_histories(args: DownstreamBalanceSheetHistoriesArgs) -> ExitCode {
    let tickers = match read_downstream_tickers() {
        Ok(values) => values,
        Err(code) => return code,
    };
    emit_json(downstream::get_balance_sheet_histories(
        args.db_path,
        &tickers,
        args.n_periods,
        &args.source,
    ))
}

fn http_client() -> Result<reqwest::blocking::Client, ExitCode> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .gzip(true)
        .build()
        .map_err(|e| {
            eprintln!("build http client: {e}");
            ExitCode::from(2)
        })
}

fn open_stocks_db(db_path: Option<PathBuf>) -> Result<rusqlite::Connection, ExitCode> {
    let path = db_path.unwrap_or_else(paths::stocks_db_path);
    edinet::storage::open_db(&path).map_err(|e| {
        eprintln!("open db: {e}");
        ExitCode::from(2)
    })
}

fn run_init_db(args: DbArgs) -> ExitCode {
    let conn = match open_stocks_db(args.db_path) {
        Ok(c) => c,
        Err(c) => return c,
    };
    // open_stocks_db already runs schema::init_db; this command is an explicit
    // re-entry point that confirms idempotent initialization.
    drop(conn);
    eprintln!("schema initialized");
    ExitCode::SUCCESS
}

fn run_parse_xbrl_financials(args: ParseXbrlFinancialsArgs) -> ExitCode {
    let db_path = args.db_path.unwrap_or_else(paths::stocks_db_path);
    let skip_existing = !args.force;
    let summary = match db::parse_xbrl_financials_to_db(
        &db_path.to_string_lossy(),
        args.ticker.as_deref(),
        args.from_ticker.as_deref(),
        skip_existing,
        args.emit_progress,
        None, // memory_restart: native binary does not self-restart
        args.parse_batch_size,
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("parse-xbrl-financials failed: {e}");
            return ExitCode::from(2);
        }
    };
    eprintln!(
        "ok={} errors={} skipped={} no_xbrl_files={} stopped_for_memory={}",
        summary.ok,
        summary.errors,
        summary.skipped,
        summary.no_xbrl_files,
        summary.stopped_for_memory,
    );
    if let Some(t) = &summary.next_from_ticker {
        eprintln!("next_from_ticker={t}");
    }
    if summary.errors > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn run_inspect_stock_db(args: InspectArgs) -> ExitCode {
    let conn = match open_stocks_db(args.db_path) {
        Ok(c) => c,
        Err(c) => return c,
    };
    let mut stmt = match conn.prepare(
        "SELECT period, statement, item_name, value, source FROM financial_items \
         WHERE ticker = ?1 ORDER BY period DESC, statement, item_name LIMIT ?2",
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("prepare: {e}");
            return ExitCode::from(2);
        }
    };
    let rows = match stmt.query_map(rusqlite::params![args.ticker, args.limit as i64], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, Option<f64>>(3)?,
            r.get::<_, String>(4)?,
        ))
    }) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("query: {e}");
            return ExitCode::from(2);
        }
    };
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut count = 0usize;
    for row in rows {
        let (period, statement, item_name, value, source) = match row {
            Ok(r) => r,
            Err(e) => {
                eprintln!("row: {e}");
                return ExitCode::from(2);
            }
        };
        let v = value
            .map(|f| f.to_string())
            .unwrap_or_else(|| "-".to_string());
        let _ = writeln!(out, "{period}\t{statement}\t{item_name}\t{v}\t{source}");
        count += 1;
    }
    eprintln!("{count} rows");
    ExitCode::SUCCESS
}

fn run_audit_xbrl_taxonomy(args: AuditArgs) -> ExitCode {
    let db_path = args.db_path.unwrap_or_else(paths::stocks_db_path);
    let min_count = args.min_count.max(1);

    let summary = match db::audit_xbrl_taxonomy(&db_path.to_string_lossy(), min_count) {
        Ok(summary) => summary,
        Err(err) => {
            eprintln!("audit-xbrl-taxonomy failed: {err}");
            return ExitCode::from(2);
        }
    };

    let report = render_audit_report(&summary, args.format, args.cf_suffix_collisions);
    if let Some(output) = &args.output {
        if let Err(err) = std::fs::write(output, &report) {
            eprintln!("failed to write {}: {err}", output.display());
            return ExitCode::from(2);
        }
    } else {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        if let Err(err) = handle.write_all(report.as_bytes()) {
            eprintln!("failed to write report: {err}");
            return ExitCode::from(2);
        }
    }

    let missing = summary.missing.len();
    let excluded = summary.excluded.len();
    let collisions = summary.cf_suffix_collisions.len();

    if args.cf_suffix_collisions {
        eprintln!(
            "Checked {} distinct concepts; {} CF suffix collisions",
            summary.checked_concepts, collisions,
        );
        return if collisions > 0 {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        };
    }

    eprintln!(
        "Checked {} distinct concepts; {} missing at min-count {}; {} excluded; {} CF suffix collisions",
        summary.checked_concepts, missing, min_count, excluded, collisions,
    );

    if missing > 0 || collisions > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn render_audit_report(
    summary: &db::TaxonomyAuditSummary,
    format: AuditFormat,
    cf_suffix_only: bool,
) -> String {
    match format {
        AuditFormat::Json => {
            let payload = if cf_suffix_only {
                serde_json::json!({
                    "checked_concepts": summary.checked_concepts,
                    "cf_suffix_collisions": summary.cf_suffix_collisions,
                })
            } else {
                serde_json::to_value(summary).expect("audit summary serializable")
            };
            serde_json::to_string_pretty(&payload).expect("audit summary JSON") + "\n"
        }
        AuditFormat::Tsv => {
            let mut out = String::new();
            if cf_suffix_only {
                for row in &summary.cf_suffix_collisions {
                    out.push_str(&format!(
                        "cf_suffix_collision\t{}\t{}\t{}\t{}\n",
                        row.statement, row.concept_name, row.canonical_name, row.reason,
                    ));
                }
            } else {
                for row in &summary.missing {
                    out.push_str(&format!(
                        "missing\t{}\t{}\t{}\t{}\n",
                        row.statement, row.concept_namespace, row.concept_name, row.count,
                    ));
                }
                for row in &summary.excluded {
                    out.push_str(&format!(
                        "excluded\t{}\t{}\t{}\t{}\n",
                        row.statement, row.concept_namespace, row.concept_name, row.count,
                    ));
                }
                for row in &summary.cf_suffix_collisions {
                    out.push_str(&format!(
                        "cf_suffix_collision\t{}\t{}\t{}\t{}\n",
                        row.statement, row.concept_name, row.canonical_name, row.reason,
                    ));
                }
            }
            out
        }
    }
}

fn run_download_xbrl(args: DownloadXbrlArgs) -> ExitCode {
    let project_root = paths::project_root();
    let api_key = match edinet::storage::get_edinet_api_key(&project_root) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let client = match http_client() {
        Ok(c) => c,
        Err(c) => return c,
    };
    let ticker = match args.ticker {
        Some(t) => t,
        None => {
            eprintln!("--ticker is required (used for the artifact path)");
            return ExitCode::from(2);
        }
    };
    let dest_dir = args
        .dest_dir
        .unwrap_or_else(|| paths::edinet_raw_dir().join("xbrl").join(&ticker));

    let xbrl_path = match edinet::client::download_xbrl_package(
        &client,
        &args.doc_id,
        &dest_dir,
        &api_key,
        Duration::from_secs(300),
    ) {
        Ok(p) => {
            println!("downloaded {} -> {}", args.doc_id, p.display());
            p
        }
        Err(e) => {
            eprintln!("download failed: {e}");
            return ExitCode::from(1);
        }
    };

    if let Some(db_path) = args.db_path {
        let conn = match edinet::storage::open_db(&db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("open db: {e}");
                return ExitCode::from(2);
            }
        };
        if let Err(e) = edinet::storage::upsert_sec_report(
            &conn,
            &ticker,
            "latest",
            &args.doc_id,
            Some(&xbrl_path.to_string_lossy()),
        ) {
            eprintln!("upsert sec_report: {e}");
            return ExitCode::from(1);
        }
    }
    ExitCode::SUCCESS
}

fn run_scrape_historical(args: ScrapeHistoricalArgs) -> ExitCode {
    let project_root = paths::project_root();
    let api_key = match edinet::storage::get_edinet_api_key(&project_root) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let conn = match open_stocks_db(args.db_path) {
        Ok(c) => c,
        Err(c) => return c,
    };

    let target_tickers: HashSet<String> = match &args.ticker {
        Some(t) => std::iter::once(t.clone()).collect(),
        None => match edinet::storage::get_edinet_target_tickers(&conn) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("load tickers: {e}");
                return ExitCode::from(2);
            }
        },
    };
    if target_tickers.is_empty() {
        eprintln!("No EDINET target tickers to process");
        return ExitCode::SUCCESS;
    }

    let skip_existing = !args.force;
    let existing_doc_ids = if skip_existing {
        match edinet::storage::get_existing_doc_ids(&conn) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("load existing doc_ids: {e}");
                return ExitCode::from(2);
            }
        }
    } else {
        HashSet::new()
    };
    if skip_existing && !existing_doc_ids.is_empty() {
        eprintln!(
            "Skipping {} already-downloaded docIDs",
            existing_doc_ids.len()
        );
    }

    let resolver = match edinet::resolver::EdinetResolver::from_db(&conn) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("build resolver: {e}");
            return ExitCode::from(2);
        }
    };

    let to_date = args
        .to_date
        .clone()
        .unwrap_or_else(|| chrono::Local::now().date_naive().to_string());
    let from_date = args.from_date.clone().unwrap_or_else(|| {
        let to = chrono::NaiveDate::parse_from_str(&to_date, "%Y-%m-%d").unwrap_or_else(|_| {
            eprintln!("invalid --to-date: {to_date}");
            std::process::exit(2);
        });
        (to - chrono::Duration::days(365 * args.years as i64)).to_string()
    });
    let interval = match args.interval {
        Some(value) if value.is_finite() && value >= 0.0 => value,
        Some(_) => {
            eprintln!("--interval must be a non-negative finite number");
            return ExitCode::from(2);
        }
        None => match config::edinet_historical_interval_seconds() {
            Ok(value) => value,
            Err(error) => {
                eprintln!("load EDINET historical interval: {error}");
                return ExitCode::from(2);
            }
        },
    };

    let client = match http_client() {
        Ok(c) => c,
        Err(c) => return c,
    };

    eprintln!(
        "Discovering {from_date}..{to_date} for {} tickers",
        target_tickers.len()
    );
    let reports = match edinet::historical::discover_historical_reports(
        &client,
        &resolver,
        &api_key,
        &from_date,
        &to_date,
        &target_tickers,
        interval,
        Duration::from_secs(30),
        &HashSet::new(),
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("discovery failed: {e}");
            return ExitCode::from(1);
        }
    };

    let total: usize = reports.values().map(|v| v.len()).sum();
    eprintln!(
        "Found {total} annual reports across {} tickers",
        reports.len()
    );
    if total == 0 {
        eprintln!("No new annual reports found");
        return ExitCode::SUCCESS;
    }

    let mut throttle = edinet::client::Throttle::new(interval);
    let raw_root = paths::edinet_raw_dir().join("xbrl");
    let mut ok = 0usize;
    let mut errors = 0usize;
    let mut skipped = 0usize;
    let mut processed = 0usize;

    for (ticker, docs) in &reports {
        for doc in docs {
            processed += 1;
            if skip_existing && existing_doc_ids.contains(&doc.doc_id) {
                skipped += 1;
                continue;
            }
            eprintln!(
                "[{processed}/{total}] {ticker}: downloading {} (FY={})",
                doc.doc_id, doc.fiscal_year
            );
            throttle.wait();
            let dest = raw_root.join(ticker);
            let xbrl_path = match edinet::client::download_xbrl_package(
                &client,
                &doc.doc_id,
                &dest,
                &api_key,
                Duration::from_secs(300),
            ) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("  XBRL download failed for {ticker} ({}): {e}", doc.doc_id);
                    errors += 1;
                    continue;
                }
            };
            if let Err(e) = edinet::storage::upsert_sec_report(
                &conn,
                ticker,
                &doc.fiscal_year,
                &doc.doc_id,
                Some(&xbrl_path.to_string_lossy()),
            ) {
                eprintln!(
                    "  upsert sec_report failed for {ticker} ({}): {e}",
                    doc.doc_id
                );
                errors += 1;
                continue;
            }
            ok += 1;
        }
    }

    eprintln!("Done: {ok} downloaded, {skipped} skipped, {errors} errors");
    if errors > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn run_sync_shikiho_forecasts(args: ShikihoArgs) -> ExitCode {
    let project_root = paths::project_root();
    let handbook_db = args
        .shikiho_db
        .unwrap_or_else(|| shikiho::sync::default_handbook_db_path(&project_root));
    let mut conn = match open_stocks_db(args.db_path) {
        Ok(c) => c,
        Err(c) => return c,
    };
    match shikiho::sync::sync_forecasts(&mut conn, &handbook_db) {
        Ok((ok, skipped)) => {
            eprintln!("Synced {ok} tickers ({skipped} skipped)");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sync-shikiho-forecasts failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn run_sync_shikiho_dividends(args: ShikihoDividendsArgs) -> ExitCode {
    let project_root = paths::project_root();
    let handbook_db = args
        .shikiho_db
        .unwrap_or_else(|| shikiho::sync::default_handbook_db_path(&project_root));
    let overrides = args
        .dividend_overrides
        .unwrap_or_else(|| paths::config_dir().join("shikiho_dividend_overrides.toml"));
    let mut conn = match open_stocks_db(args.db_path) {
        Ok(c) => c,
        Err(c) => return c,
    };
    match shikiho::sync::sync_dividends(&mut conn, &handbook_db, &overrides) {
        Ok((ok, skipped)) => {
            eprintln!("Synced {ok} tickers ({skipped} skipped)");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sync-shikiho-dividends failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn run_compute_eps(args: DbArgs) -> ExitCode {
    let mut conn = match open_stocks_db(args.db_path) {
        Ok(c) => c,
        Err(c) => return c,
    };
    match shikiho::eps::compute_eps(&mut conn) {
        Ok((tickers, items)) => {
            eprintln!("Computed EPS for {tickers} tickers ({items} items)");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("compute-eps failed: {e}");
            ExitCode::from(1)
        }
    }
}
