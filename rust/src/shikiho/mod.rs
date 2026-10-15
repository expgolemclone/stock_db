//! Shikiho (四季報) sync: copy forecasts/dividends from japan_company_handbook
//! into stocks.db financial_values (source=shikiho), and compute EPS
//! (source=computed).

pub mod eps;
pub mod storage;
pub mod sync;
