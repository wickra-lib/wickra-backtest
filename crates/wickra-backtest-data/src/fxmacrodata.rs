//! `FXMacroData` REST helpers for macro, FX, calendar, COT and related datasets.
//!
//! The client builds stable endpoint URLs, provides a raw JSON escape hatch for
//! broad endpoint coverage, and includes typed parsers for the calendar and COT
//! payloads that are commonly useful as external backtest feeds.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use wickra_backtest_core::{BacktestError, Result};

/// Supported `FXMacroData` REST endpoint families.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FxMacroDataEndpoint {
    /// Macro indicator catalogue for one currency.
    DataCatalogue,
    /// Historical announcement rows for one currency and indicator.
    Announcements,
    /// Latest announcement row for one currency.
    LatestAnnouncements,
    /// Recent announcement-change feed.
    AnnouncementChanges,
    /// Economic release calendar for one currency.
    Calendar,
    /// Forecast, nowcast, survey or consensus rows.
    Predictions,
    /// Historical FX spot-rate rows.
    Forex,
    /// CFTC Commitment of Traders positioning.
    Cot,
    /// One commodity history.
    Commodity,
    /// Latest commodity values.
    CommoditiesLatest,
    /// Yield or rate curve nodes.
    Curves,
    /// Curve slope proxy series.
    CurveProxies,
    /// Forward curve rows.
    ForwardCurves,
    /// Historical rate differentials for a currency pair.
    RateDifferentials,
    /// Forward-rate differentials for a currency pair.
    ForwardDifferentials,
    /// Global FX market-session state.
    MarketSessions,
    /// Risk-on/risk-off sentiment readings.
    RiskSentiment,
    /// Central-bank or macro news rows.
    News,
    /// Central-bank press-release rows.
    PressReleases,
    /// `FXMacroData` GraphQL endpoint.
    Graphql,
    /// Caller-supplied endpoint path.
    Custom,
}

/// Request descriptor used to build an `FXMacroData` URL or POST body.
#[derive(Clone, Debug)]
pub struct FxMacroDataRequest {
    /// Endpoint family to query.
    pub endpoint: FxMacroDataEndpoint,
    /// ISO 4217 currency code for currency-scoped endpoints.
    pub currency: Option<String>,
    /// Indicator, commodity or series slug for endpoint families that need one.
    pub indicator: Option<String>,
    /// Base currency for pair-scoped endpoints.
    pub base: Option<String>,
    /// Quote currency for pair-scoped endpoints.
    pub quote: Option<String>,
    /// Custom path used when [`FxMacroDataEndpoint::Custom`] is selected.
    pub path: Option<String>,
    /// Query parameters appended to the endpoint URL.
    pub params: Vec<(String, String)>,
    /// JSON body used for GraphQL or custom POST requests.
    pub body: Option<serde_json::Value>,
}

impl FxMacroDataRequest {
    /// Create an empty request for an endpoint family.
    #[must_use]
    pub fn new(endpoint: FxMacroDataEndpoint) -> Self {
        Self {
            endpoint,
            currency: None,
            indicator: None,
            base: None,
            quote: None,
            path: None,
            params: Vec::new(),
            body: None,
        }
    }
}

/// Typed economic release-calendar response.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FxMacroDataCalendarResponse {
    /// Calendar rows extracted from a top-level array or a `data`/`items`/`results`/`rows` envelope.
    pub rows: Vec<FxMacroDataCalendarRow>,
}

/// One economic release-calendar row.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FxMacroDataCalendarRow {
    /// Currency code associated with the release.
    #[serde(default)]
    pub currency: Option<String>,
    /// Indicator slug or display name associated with the release.
    #[serde(default)]
    pub indicator: Option<String>,
    /// Release date when the API provides a date-only field.
    #[serde(default)]
    pub release_date: Option<String>,
    /// Announcement timestamp when the API provides an exact datetime field.
    #[serde(default)]
    pub announcement_datetime: Option<String>,
    /// Actual released value, retained as JSON to preserve API numeric/string shape.
    #[serde(default)]
    pub actual: Option<serde_json::Value>,
    /// Consensus or forecast value, retained as JSON to preserve API numeric/string shape.
    #[serde(default)]
    pub consensus: Option<serde_json::Value>,
    /// Previous value, retained as JSON to preserve API numeric/string shape.
    #[serde(default)]
    pub previous: Option<serde_json::Value>,
    /// Additional fields returned by `FXMacroData`.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Typed CFTC Commitment of Traders response.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FxMacroDataCotResponse {
    /// COT rows extracted from a top-level array or a `data`/`items`/`results`/`rows` envelope.
    pub rows: Vec<FxMacroDataCotRow>,
}

/// One CFTC Commitment of Traders row.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FxMacroDataCotRow {
    /// Currency code associated with the futures contract.
    #[serde(default)]
    pub currency: Option<String>,
    /// Report date for the COT row.
    #[serde(default)]
    pub report_date: Option<String>,
    /// Contract or market name.
    #[serde(default)]
    pub market: Option<String>,
    /// Net positioning value, retained as JSON to preserve API numeric/string shape.
    #[serde(default)]
    pub net_position: Option<serde_json::Value>,
    /// Non-commercial long positioning, retained as JSON to preserve API numeric/string shape.
    #[serde(default)]
    pub noncommercial_long: Option<serde_json::Value>,
    /// Non-commercial short positioning, retained as JSON to preserve API numeric/string shape.
    #[serde(default)]
    pub noncommercial_short: Option<serde_json::Value>,
    /// Additional fields returned by `FXMacroData`.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Minimal `FXMacroData` REST client.
#[derive(Clone, Debug)]
pub struct FxMacroDataClient {
    base_url: String,
    api_key: Option<String>,
}

impl Default for FxMacroDataClient {
    fn default() -> Self {
        Self::new(None)
    }
}

impl FxMacroDataClient {
    /// Create a client with the default `FXMacroData` API base URL.
    #[must_use]
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_base_url(api_key, "https://api.fxmacrodata.com/v1")
    }

    /// Create a client using `FXMACRODATA_API_KEY` or `FXMD_API_KEY` from the environment.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(api_key_from_env())
    }

    /// Create a client with an explicit base URL.
    #[must_use]
    pub fn with_base_url(api_key: Option<String>, base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            api_key,
        }
    }

    /// Build the URL for a request without sending it.
    pub fn build_url(&self, request: &FxMacroDataRequest) -> Result<String> {
        let mut params = request.params.clone();
        if let Some(api_key) = &self.api_key {
            if !params.iter().any(|(key, _)| key == "api_key") {
                params.push(("api_key".to_owned(), api_key.clone()));
            }
        }

        let mut url = format!("{}{}", self.base_url, Self::path(request)?);
        if !params.is_empty() {
            url.push('?');
            url.push_str(
                &params
                    .iter()
                    .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
                    .collect::<Vec<_>>()
                    .join("&"),
            );
        }
        Ok(url)
    }

    /// Fetch an `FXMacroData` endpoint and deserialize it as raw JSON.
    #[cfg(feature = "fxmacrodata")]
    pub fn fetch_json(&self, request: FxMacroDataRequest) -> Result<serde_json::Value> {
        let url = self.build_url(&request)?;
        if matches!(request.endpoint, FxMacroDataEndpoint::Graphql)
            || (matches!(request.endpoint, FxMacroDataEndpoint::Custom) && request.body.is_some())
        {
            let body = request.body.unwrap_or(serde_json::Value::Null).to_string();
            let text = ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_string(&body)
                .map_err(|e| fx_error(format!("request failed: {e}")))?
                .into_string()
                .map_err(|e| fx_error(format!("response: {e}")))?;
            return serde_json::from_str(&text).map_err(|e| fx_error(format!("JSON: {e}")));
        }

        let text = ureq::get(&url)
            .call()
            .map_err(|e| fx_error(format!("request failed: {e}")))?
            .into_string()
            .map_err(|e| fx_error(format!("response: {e}")))?;
        serde_json::from_str(&text).map_err(|e| fx_error(format!("JSON: {e}")))
    }

    /// Fetch and parse a typed economic release-calendar response.
    #[cfg(feature = "fxmacrodata")]
    pub fn fetch_calendar(&self, currency: &str) -> Result<FxMacroDataCalendarResponse> {
        parse_calendar_value(self.fetch_json(self.calendar(currency))?)
    }

    /// Fetch and parse a typed CFTC COT response.
    #[cfg(feature = "fxmacrodata")]
    pub fn fetch_cot(&self, currency: &str) -> Result<FxMacroDataCotResponse> {
        parse_cot_value(self.fetch_json(self.cot(currency))?)
    }

    /// Build a release-calendar request for a currency.
    #[must_use]
    pub fn calendar(&self, currency: &str) -> FxMacroDataRequest {
        let mut request = FxMacroDataRequest::new(FxMacroDataEndpoint::Calendar);
        request.currency = Some(currency.to_owned());
        request
    }

    /// Build a predictions request for a currency and indicator.
    #[must_use]
    pub fn predictions(&self, currency: &str, indicator: &str) -> FxMacroDataRequest {
        let mut request = FxMacroDataRequest::new(FxMacroDataEndpoint::Predictions);
        request.currency = Some(currency.to_owned());
        request.indicator = Some(indicator.to_owned());
        request
    }

    /// Build a historical FX spot-rate request.
    #[must_use]
    pub fn forex(&self, base: &str, quote: &str) -> FxMacroDataRequest {
        let mut request = FxMacroDataRequest::new(FxMacroDataEndpoint::Forex);
        request.base = Some(base.to_owned());
        request.quote = Some(quote.to_owned());
        request
    }

    /// Build a CFTC COT request for a currency.
    #[must_use]
    pub fn cot(&self, currency: &str) -> FxMacroDataRequest {
        let mut request = FxMacroDataRequest::new(FxMacroDataEndpoint::Cot);
        request.currency = Some(currency.to_owned());
        request
    }

    fn path(request: &FxMacroDataRequest) -> Result<String> {
        let path = match request.endpoint {
            FxMacroDataEndpoint::DataCatalogue => {
                format!(
                    "/data_catalogue/{}",
                    segment(request.currency.as_ref(), "currency")?
                )
            }
            FxMacroDataEndpoint::Announcements => format!(
                "/announcements/{}/{}",
                segment(request.currency.as_ref(), "currency")?,
                segment(request.indicator.as_ref(), "indicator")?
            ),
            FxMacroDataEndpoint::LatestAnnouncements => {
                format!(
                    "/announcements/{}/latest",
                    segment(request.currency.as_ref(), "currency")?
                )
            }
            FxMacroDataEndpoint::AnnouncementChanges => "/announcements/changes".to_owned(),
            FxMacroDataEndpoint::Calendar => {
                format!(
                    "/calendar/{}",
                    segment(request.currency.as_ref(), "currency")?
                )
            }
            FxMacroDataEndpoint::Predictions => format!(
                "/predictions/{}/{}",
                segment(request.currency.as_ref(), "currency")?,
                segment(request.indicator.as_ref(), "indicator")?
            ),
            FxMacroDataEndpoint::Forex => format!(
                "/forex/{}/{}",
                segment(request.base.as_ref(), "base")?,
                segment(request.quote.as_ref(), "quote")?
            ),
            FxMacroDataEndpoint::Cot => {
                format!("/cot/{}", segment(request.currency.as_ref(), "currency")?)
            }
            FxMacroDataEndpoint::Commodity => {
                format!(
                    "/commodities/{}",
                    segment(request.indicator.as_ref(), "indicator")?
                )
            }
            FxMacroDataEndpoint::CommoditiesLatest => "/commodities/latest".to_owned(),
            FxMacroDataEndpoint::Curves => {
                format!(
                    "/curves/{}",
                    segment(request.currency.as_ref(), "currency")?
                )
            }
            FxMacroDataEndpoint::CurveProxies => {
                format!(
                    "/curve_proxies/{}",
                    segment(request.currency.as_ref(), "currency")?
                )
            }
            FxMacroDataEndpoint::ForwardCurves => {
                format!(
                    "/forward_curves/{}",
                    segment(request.currency.as_ref(), "currency")?
                )
            }
            FxMacroDataEndpoint::RateDifferentials => format!(
                "/rate_differentials/{}/{}",
                segment(request.base.as_ref(), "base")?,
                segment(request.quote.as_ref(), "quote")?
            ),
            FxMacroDataEndpoint::ForwardDifferentials => format!(
                "/forward_differentials/{}/{}",
                segment(request.base.as_ref(), "base")?,
                segment(request.quote.as_ref(), "quote")?
            ),
            FxMacroDataEndpoint::MarketSessions => "/market_sessions".to_owned(),
            FxMacroDataEndpoint::RiskSentiment => "/risk_sentiment".to_owned(),
            FxMacroDataEndpoint::News => {
                format!("/news/{}", segment(request.currency.as_ref(), "currency")?)
            }
            FxMacroDataEndpoint::PressReleases => {
                format!(
                    "/press-releases/{}",
                    segment(request.currency.as_ref(), "currency")?
                )
            }
            FxMacroDataEndpoint::Graphql => "/graphql".to_owned(),
            FxMacroDataEndpoint::Custom => request
                .path
                .as_deref()
                .map(|path| {
                    if path.starts_with('/') {
                        path.to_owned()
                    } else {
                        format!("/{path}")
                    }
                })
                .ok_or_else(|| fx_error("path is required"))?,
        };
        Ok(path)
    }
}

/// Parse a JSON string into a typed economic release-calendar response.
pub fn parse_calendar_json(json: &str) -> Result<FxMacroDataCalendarResponse> {
    let value = parse_json_value(json, "calendar")?;
    parse_calendar_value(value)
}

/// Parse a raw JSON value into a typed economic release-calendar response.
pub fn parse_calendar_value(value: serde_json::Value) -> Result<FxMacroDataCalendarResponse> {
    Ok(FxMacroDataCalendarResponse {
        rows: parse_typed_rows(value, "calendar")?,
    })
}

/// Parse a JSON string into a typed CFTC COT response.
pub fn parse_cot_json(json: &str) -> Result<FxMacroDataCotResponse> {
    let value = parse_json_value(json, "cot")?;
    parse_cot_value(value)
}

/// Parse a raw JSON value into a typed CFTC COT response.
pub fn parse_cot_value(value: serde_json::Value) -> Result<FxMacroDataCotResponse> {
    Ok(FxMacroDataCotResponse {
        rows: parse_typed_rows(value, "cot")?,
    })
}

fn api_key_from_env() -> Option<String> {
    std::env::var("FXMACRODATA_API_KEY")
        .or_else(|_| std::env::var("FXMD_API_KEY"))
        .ok()
        .filter(|key| !key.trim().is_empty())
}

fn parse_json_value(json: &str, context: &str) -> Result<serde_json::Value> {
    serde_json::from_str(json).map_err(|e| fx_error(format!("{context} JSON: {e}")))
}

fn parse_typed_rows<T>(value: serde_json::Value, context: &str) -> Result<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    extract_rows(value, context)?
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            serde_json::from_value(row).map_err(|e| fx_error(format!("{context} row {index}: {e}")))
        })
        .collect()
}

fn extract_rows(value: serde_json::Value, context: &str) -> Result<Vec<serde_json::Value>> {
    match value {
        serde_json::Value::Array(rows) => Ok(rows),
        serde_json::Value::Object(mut object) => {
            let row_source = ["data", "items", "results", "rows"]
                .into_iter()
                .find_map(|key| object.remove(key).map(|payload| (key, payload)));
            match row_source {
                Some((_, serde_json::Value::Array(rows))) => Ok(rows),
                Some((key, _)) => Err(fx_error(format!(
                    "{context} field `{key}` must be an array"
                ))),
                None => Err(fx_error(format!(
                    "{context} response must be an array or contain a data/items/results/rows array"
                ))),
            }
        }
        _ => Err(fx_error(format!(
            "{context} response must be an array or object"
        ))),
    }
}

fn segment(value: Option<&String>, name: &str) -> Result<String> {
    value
        .map(|value| encode(&value.to_lowercase()))
        .ok_or_else(|| fx_error(format!("{name} is required")))
}

fn fx_error(message: impl std::fmt::Display) -> BacktestError {
    BacktestError::InvalidData(format!("fxmacrodata: {message}"))
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, sync::Mutex};

    use serde_json::json;

    use super::{
        parse_calendar_json, parse_cot_json, FxMacroDataClient, FxMacroDataEndpoint,
        FxMacroDataRequest,
    };

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvSnapshot {
        name: &'static str,
        value: Option<OsString>,
    }

    impl EnvSnapshot {
        fn capture(name: &'static str) -> Self {
            Self {
                name,
                value: std::env::var_os(name),
            }
        }
    }

    impl Drop for EnvSnapshot {
        fn drop(&mut self) {
            match &self.value {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
        }
    }

    #[test]
    fn builds_authenticated_macro_urls() {
        let client = FxMacroDataClient::with_base_url(
            Some("test-key".to_owned()),
            "https://api.fxmacrodata.com/v1/",
        );
        let mut request = client.predictions("USD", "non_farm_payrolls");
        request.params.push(("limit".to_owned(), "1".to_owned()));

        assert_eq!(
            client.build_url(&request).unwrap(),
            "https://api.fxmacrodata.com/v1/predictions/usd/non_farm_payrolls?limit=1&api_key=test-key"
        );
    }

    #[test]
    fn builds_cross_currency_market_urls() {
        let client = FxMacroDataClient::new(Some("test-key".to_owned()));
        let mut request = FxMacroDataRequest::new(FxMacroDataEndpoint::RateDifferentials);
        request.base = Some("EUR".to_owned());
        request.quote = Some("USD".to_owned());
        request.params.push(("tenor".to_owned(), "2y".to_owned()));

        assert_eq!(
            client.build_url(&request).unwrap(),
            "https://api.fxmacrodata.com/v1/rate_differentials/eur/usd?tenor=2y&api_key=test-key"
        );
    }

    #[test]
    fn default_does_not_read_environment() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _primary = EnvSnapshot::capture("FXMACRODATA_API_KEY");
        let _fallback = EnvSnapshot::capture("FXMD_API_KEY");
        std::env::set_var("FXMACRODATA_API_KEY", "env-key");
        std::env::set_var("FXMD_API_KEY", "fallback-key");

        let client = FxMacroDataClient::default();

        assert_eq!(
            client.build_url(&client.calendar("USD")).unwrap(),
            "https://api.fxmacrodata.com/v1/calendar/usd"
        );
    }

    #[test]
    fn from_env_reads_explicit_api_key() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _primary = EnvSnapshot::capture("FXMACRODATA_API_KEY");
        let _fallback = EnvSnapshot::capture("FXMD_API_KEY");
        std::env::set_var("FXMACRODATA_API_KEY", "env-key");
        std::env::set_var("FXMD_API_KEY", "fallback-key");

        let client = FxMacroDataClient::from_env();

        assert_eq!(
            client.build_url(&client.calendar("USD")).unwrap(),
            "https://api.fxmacrodata.com/v1/calendar/usd?api_key=env-key"
        );
    }

    #[test]
    fn rejects_missing_required_path_segments() {
        let client = FxMacroDataClient::new(None);
        let calendar = FxMacroDataRequest::new(FxMacroDataEndpoint::Calendar);
        let prediction = FxMacroDataRequest::new(FxMacroDataEndpoint::Predictions);
        let custom = FxMacroDataRequest::new(FxMacroDataEndpoint::Custom);

        assert_eq!(
            client.build_url(&calendar).unwrap_err().to_string(),
            "invalid input data: fxmacrodata: currency is required"
        );
        assert_eq!(
            client.build_url(&prediction).unwrap_err().to_string(),
            "invalid input data: fxmacrodata: currency is required"
        );
        assert_eq!(
            client.build_url(&custom).unwrap_err().to_string(),
            "invalid input data: fxmacrodata: path is required"
        );
    }

    #[test]
    fn parses_calendar_envelopes_as_typed_rows() {
        let response = parse_calendar_json(
            r#"{"data":[{"currency":"USD","indicator":"cpi","release_date":"2026-07-15","actual":3.0,"source":"BLS"}]}"#,
        )
        .unwrap();

        assert_eq!(response.rows.len(), 1);
        assert_eq!(response.rows[0].currency.as_deref(), Some("USD"));
        assert_eq!(response.rows[0].actual, Some(json!(3.0)));
        assert_eq!(response.rows[0].extra.get("source"), Some(&json!("BLS")));
    }

    #[test]
    fn parses_cot_arrays_as_typed_rows() {
        let response = parse_cot_json(
            r#"[{"currency":"JPY","report_date":"2026-07-07","net_position":-123,"noncommercial_long":10}]"#,
        )
        .unwrap();

        assert_eq!(response.rows.len(), 1);
        assert_eq!(response.rows[0].currency.as_deref(), Some("JPY"));
        assert_eq!(response.rows[0].net_position, Some(json!(-123)));
        assert_eq!(response.rows[0].noncommercial_long, Some(json!(10)));
    }
}
