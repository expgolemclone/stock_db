//! Runtime configuration loading from config/*.toml.

use std::path::PathBuf;

use crate::paths;

#[derive(Debug, serde::Deserialize)]
struct MagicNumbersConfig {
    edinet_historical: EdinetHistoricalConfig,
}

#[derive(Debug, serde::Deserialize)]
struct EdinetHistoricalConfig {
    interval_seconds: f64,
}

pub fn config_path(name: &str) -> PathBuf {
    paths::config_dir().join(name)
}

/// Read a TOML file from config/ and parse it into a toml::Value.
pub fn load_toml(name: &str) -> Result<toml::Value, String> {
    let path = config_path(name);
    let text = std::fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    text.parse::<toml::Value>()
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

pub fn edinet_historical_interval_seconds() -> Result<f64, String> {
    let path = config_path("magic_numbers.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let config: MagicNumbersConfig = toml::from_str(&text)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    let interval = config.edinet_historical.interval_seconds;
    if !interval.is_finite() || interval <= 0.0 {
        return Err(format!(
            "{}.edinet_historical.interval_seconds must be a positive finite number",
            path.display()
        ));
    }
    Ok(interval)
}
