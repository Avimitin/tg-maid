// ----------------- Types ----------------
use crate::data::{AppData, Sendable};
use serde::Deserialize;
use std::collections::HashMap;

/// The actual rate information during the runtime
#[derive(Debug)]
pub struct CurrencyRateInfo {
    pub date: String,
    pub rate: f64,
}

impl CurrencyRateInfo {
    pub fn new(date: String, rate: f64) -> Self {
        Self { date, rate }
    }
}

/// Possible value return from API
#[derive(Deserialize)]
#[serde(untagged)]
pub enum CurrencyV1PossibleResponse {
    Float(f64),
    String(String),
}

impl CurrencyV1PossibleResponse {
    pub fn unwrap_rate(&self) -> f64 {
        match self {
            Self::Float(f) => *f,
            _ => panic!("currency return non-float rate"),
        }
    }

    pub fn unwrap_date(&self) -> String {
        match self {
            Self::String(s) => s.to_string(),
            _ => panic!("currency return non-string date"),
        }
    }
}

pub async fn exchange(
    data: AppData,
    amount: f64,
    from: &str,
    to: &str,
) -> anyhow::Result<Sendable> {
    let rate = fetch_rate(&data, from, to).await?;

    let from_fullname = get_fullname(&data, from).await?;
    let to_fullname = get_fullname(&data, to).await?;

    let display = format!(
        r#"
<b>{}</b> --> <b>{}</b>

<b>{:.3}</b> {} = <b>{:.3}</b> {}

Date: {}
                           "#,
        from_fullname,
        to_fullname,
        amount,
        from.to_uppercase(),
        rate.rate * amount,
        to.to_uppercase(),
        rate.date
    );

    Ok(Sendable::text(display))
}

async fn get_fullname(data: &AppData, code: &str) -> anyhow::Result<String> {
    use redis::Commands;
    const CACHE_KEY: &str = "AVAILABLE_CURRENCIES";
    let fullname: Option<String> = data.cacher.get_conn().hget(CACHE_KEY, code)?;
    if let Some(fullname) = fullname {
        return Ok(fullname);
    }

    const available_currencies: [&str; 2] = [
        "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies.min.json",
        "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies.min.json",
    ];

    let mut trace = Vec::with_capacity(2);

    for url in available_currencies {
        match data.requester.to_t::<HashMap<String, String>>(url).await {
            Ok(map) => {
                let mut conn = data.cacher.get_conn();
                for (k, v) in map {
                    conn.hset(CACHE_KEY, k, v)?;
                }
            }
            Err(err) => trace.push(err.to_string()),
        }
    }

    anyhow::bail!("All API fail: {}", trace.join("\n"))
}

async fn fetch_rate(data: &AppData, from: &str, to: &str) -> anyhow::Result<CurrencyRateInfo> {
    const fallbacks_urls: [&str; 2] = [
          "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies/{from}/{to}.min.json",
          "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies/{from}/{to}.min.json",
    ];

    let mut error_trace = Vec::new();

    for url in &fallbacks_urls {
        let url = format!("{}/{}/{}.min.json", url, from, to);

        match data
            .requester
            .to_t::<HashMap<String, CurrencyV1PossibleResponse>>(url)
            .await
        {
            Ok(res) => {
                let rate = res
                    .get(to)
                    .ok_or_else(|| anyhow::anyhow!("missing `{to}` info from currency api v1"))?
                    .unwrap_rate();
                let date = res
                    .get("date")
                    .ok_or_else(|| anyhow::anyhow!("missing `date` info from currency api v1"))?
                    .unwrap_date();
                return Ok(CurrencyRateInfo::new(date, rate));
            }
            Err(e) => {
                error_trace.push(e.to_string());
            }
        }
    }

    anyhow::bail!(
        "fail to send request to all currency API: {}",
        error_trace.join("\n\n")
    )
}
