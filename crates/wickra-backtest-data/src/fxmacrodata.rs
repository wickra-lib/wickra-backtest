use wickra_backtest_core::{BacktestError, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FxMacroDataEndpoint {
    DataCatalogue,
    Announcements,
    LatestAnnouncements,
    AnnouncementChanges,
    Calendar,
    Predictions,
    Forex,
    Cot,
    Commodity,
    CommoditiesLatest,
    Curves,
    CurveProxies,
    ForwardCurves,
    RateDifferentials,
    ForwardDifferentials,
    MarketSessions,
    RiskSentiment,
    News,
    PressReleases,
    Graphql,
    Custom,
}

#[derive(Clone, Debug)]
pub struct FxMacroDataRequest {
    pub endpoint: FxMacroDataEndpoint,
    pub currency: Option<String>,
    pub indicator: Option<String>,
    pub base: Option<String>,
    pub quote: Option<String>,
    pub path: Option<String>,
    pub params: Vec<(String, String)>,
    pub body: Option<serde_json::Value>,
}

impl FxMacroDataRequest {
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

#[derive(Clone, Debug)]
pub struct FxMacroDataClient {
    base_url: String,
    api_key: Option<String>,
}

impl Default for FxMacroDataClient {
    fn default() -> Self {
        Self::new(
            std::env::var("FXMACRODATA_API_KEY")
                .or_else(|_| std::env::var("FXMD_API_KEY"))
                .ok(),
        )
    }
}

impl FxMacroDataClient {
    #[must_use]
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_base_url(api_key, "https://api.fxmacrodata.com/v1")
    }

    #[must_use]
    pub fn with_base_url(api_key: Option<String>, base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            api_key,
        }
    }

    pub fn build_url(&self, request: &FxMacroDataRequest) -> Result<String> {
        let mut params = request.params.clone();
        if let Some(api_key) = &self.api_key {
            if !params.iter().any(|(key, _)| key == "api_key") {
                params.push(("api_key".to_owned(), api_key.clone()));
            }
        }

        let mut url = format!("{}{}", self.base_url, self.path(request)?);
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
                .map_err(|e| {
                    BacktestError::InvalidData(format!("FXMacroData request failed: {e}"))
                })?
                .into_string()
                .map_err(|e| BacktestError::InvalidData(format!("FXMacroData response: {e}")))?;
            return serde_json::from_str(&text)
                .map_err(|e| BacktestError::InvalidData(format!("FXMacroData JSON: {e}")));
        }

        let text = ureq::get(&url)
            .call()
            .map_err(|e| BacktestError::InvalidData(format!("FXMacroData request failed: {e}")))?
            .into_string()
            .map_err(|e| BacktestError::InvalidData(format!("FXMacroData response: {e}")))?;
        serde_json::from_str(&text)
            .map_err(|e| BacktestError::InvalidData(format!("FXMacroData JSON: {e}")))
    }

    pub fn calendar(&self, currency: &str) -> FxMacroDataRequest {
        let mut request = FxMacroDataRequest::new(FxMacroDataEndpoint::Calendar);
        request.currency = Some(currency.to_owned());
        request
    }

    pub fn predictions(&self, currency: &str, indicator: &str) -> FxMacroDataRequest {
        let mut request = FxMacroDataRequest::new(FxMacroDataEndpoint::Predictions);
        request.currency = Some(currency.to_owned());
        request.indicator = Some(indicator.to_owned());
        request
    }

    pub fn forex(&self, base: &str, quote: &str) -> FxMacroDataRequest {
        let mut request = FxMacroDataRequest::new(FxMacroDataEndpoint::Forex);
        request.base = Some(base.to_owned());
        request.quote = Some(quote.to_owned());
        request
    }

    fn path(&self, request: &FxMacroDataRequest) -> Result<String> {
        let path = match request.endpoint {
            FxMacroDataEndpoint::DataCatalogue => {
                format!(
                    "/data_catalogue/{}",
                    segment(&request.currency, "currency")?
                )
            }
            FxMacroDataEndpoint::Announcements => format!(
                "/announcements/{}/{}",
                segment(&request.currency, "currency")?,
                segment(&request.indicator, "indicator")?
            ),
            FxMacroDataEndpoint::LatestAnnouncements => {
                format!(
                    "/announcements/{}/latest",
                    segment(&request.currency, "currency")?
                )
            }
            FxMacroDataEndpoint::AnnouncementChanges => "/announcements/changes".to_owned(),
            FxMacroDataEndpoint::Calendar => {
                format!("/calendar/{}", segment(&request.currency, "currency")?)
            }
            FxMacroDataEndpoint::Predictions => format!(
                "/predictions/{}/{}",
                segment(&request.currency, "currency")?,
                segment(&request.indicator, "indicator")?
            ),
            FxMacroDataEndpoint::Forex => format!(
                "/forex/{}/{}",
                segment(&request.base, "base")?,
                segment(&request.quote, "quote")?
            ),
            FxMacroDataEndpoint::Cot => format!("/cot/{}", segment(&request.currency, "currency")?),
            FxMacroDataEndpoint::Commodity => {
                format!("/commodities/{}", segment(&request.indicator, "indicator")?)
            }
            FxMacroDataEndpoint::CommoditiesLatest => "/commodities/latest".to_owned(),
            FxMacroDataEndpoint::Curves => {
                format!("/curves/{}", segment(&request.currency, "currency")?)
            }
            FxMacroDataEndpoint::CurveProxies => {
                format!("/curve_proxies/{}", segment(&request.currency, "currency")?)
            }
            FxMacroDataEndpoint::ForwardCurves => {
                format!(
                    "/forward_curves/{}",
                    segment(&request.currency, "currency")?
                )
            }
            FxMacroDataEndpoint::RateDifferentials => format!(
                "/rate_differentials/{}/{}",
                segment(&request.base, "base")?,
                segment(&request.quote, "quote")?
            ),
            FxMacroDataEndpoint::ForwardDifferentials => format!(
                "/forward_differentials/{}/{}",
                segment(&request.base, "base")?,
                segment(&request.quote, "quote")?
            ),
            FxMacroDataEndpoint::MarketSessions => "/market_sessions".to_owned(),
            FxMacroDataEndpoint::RiskSentiment => "/risk_sentiment".to_owned(),
            FxMacroDataEndpoint::News => {
                format!("/news/{}", segment(&request.currency, "currency")?)
            }
            FxMacroDataEndpoint::PressReleases => {
                format!(
                    "/press-releases/{}",
                    segment(&request.currency, "currency")?
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
                .ok_or_else(|| BacktestError::InvalidData("FXMacroData path is required".into()))?,
        };
        Ok(path)
    }
}

fn segment(value: &Option<String>, name: &str) -> Result<String> {
    value
        .as_deref()
        .map(str::to_lowercase)
        .map(|value| encode(&value))
        .ok_or_else(|| BacktestError::InvalidData(format!("FXMacroData {name} is required")))
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
    use super::{FxMacroDataClient, FxMacroDataEndpoint, FxMacroDataRequest};

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
}
