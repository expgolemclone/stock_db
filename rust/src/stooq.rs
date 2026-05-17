use std::fs;

use chrono::NaiveDate;

const EXPECTED_HEADER: [&str; 10] = [
    "<ticker>",
    "<per>",
    "<date>",
    "<time>",
    "<open>",
    "<high>",
    "<low>",
    "<close>",
    "<vol>",
    "<openint>",
];

pub fn parse_daily_file(path: &str) -> Result<Vec<(String, String, f64)>, String> {
    let content = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
    let mut lines = content.lines();
    let Some(header_line) = lines.next() else {
        return Err("Stooq daily file is empty".to_string());
    };
    let header = split_row(header_line);
    let normalized_header = header
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    if normalized_header != EXPECTED_HEADER {
        return Err(format!("Unexpected Stooq header: {header:?}"));
    }

    let mut rows = Vec::new();
    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        let row = split_row(line);
        if row.is_empty() || row.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }
        if row.len() != header.len() {
            return Err(format!(
                "Unexpected column count on line {line_number}: {}",
                row.len()
            ));
        }
        let symbol = row[0].trim().to_ascii_uppercase();
        let Some(ticker) = symbol.strip_suffix(".JP") else {
            continue;
        };
        if !is_supported_ticker(ticker) {
            continue;
        }
        let price_date = row[2].trim();
        if price_date.is_empty() {
            continue;
        }
        let close_raw = row[7].trim();
        if close_raw.is_empty() {
            continue;
        }
        let normalized_date = NaiveDate::parse_from_str(price_date, "%Y%m%d")
            .map_err(|_| format!("Invalid Date value on line {line_number}: {price_date:?}"))?
            .format("%Y-%m-%d")
            .to_string();
        let close = close_raw
            .parse::<f64>()
            .map_err(|_| format!("Invalid Close value on line {line_number}: {close_raw:?}"))?;
        rows.push((ticker.to_string(), normalized_date, close));
    }
    if rows.is_empty() {
        return Err(format!("No JP daily prices found in {path}"));
    }
    Ok(rows)
}

fn split_row(line: &str) -> Vec<String> {
    line.split(',').map(ToOwned::to_owned).collect()
}

fn is_supported_ticker(ticker: &str) -> bool {
    let bytes = ticker.as_bytes();
    match bytes.len() {
        4 | 5 if bytes.iter().all(u8::is_ascii_digit) => true,
        4 => bytes[..3].iter().all(u8::is_ascii_digit) && bytes[3].is_ascii_uppercase(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_supported_ticker;

    #[test]
    fn supported_ticker_shapes_match_python_contract() {
        assert!(is_supported_ticker("7203"));
        assert!(is_supported_ticker("12345"));
        assert!(is_supported_ticker("211A"));
        assert!(!is_supported_ticker("AAPL"));
        assert!(!is_supported_ticker("12AB"));
    }
}
