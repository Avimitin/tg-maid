use super::Sendable;
use crate::app::AppData;
use serde::Deserialize;
use std::collections::HashMap;

pub type CurrencyMapping = HashMap<String, f64>;

/// The actual rate information during the runtime
#[derive(Debug, Deserialize)]
pub struct CurrencyRateInfo {
    pub date: String,

    #[serde(flatten)]
    pub payload: HashMap<String, CurrencyMapping>,
}

pub async fn exchange(
    data: AppData,
    amount: f64,
    from: &str,
    to: &str,
) -> anyhow::Result<Sendable> {
    let data = fetch_rate(&data, from).await?;

    let all_rate = data
        .payload
        .get(from)
        .ok_or_else(|| anyhow::anyhow!("{from} not found"))?;

    let rate = all_rate
        .get(to)
        .ok_or_else(|| anyhow::anyhow!("${to} not found"))?;

    let display = format!(
        r#"
<b>{:.3}</b> {} = <b>{:.3}</b> {}

Date: {}"#,
        amount,
        from.to_uppercase(),
        rate * amount,
        to.to_uppercase(),
        data.date
    );

    Ok(Sendable::text(display))
}

async fn fetch_rate(data: &AppData, from: &str) -> anyhow::Result<CurrencyRateInfo> {
    const FALLBACKS: [&str; 2] = [
        "https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies",
        "https://latest.currency-api.pages.dev/v1/currencies",
    ];

    let mut error_trace = Vec::new();

    for url in &FALLBACKS {
        let url = format!("{}/{}.min.json", url, from);

        match data.requester.to_t::<CurrencyRateInfo>(url).await {
            Err(e) => {
                error_trace.push(format!("{:?}", e));
            }
            resp => return resp,
        }
    }

    anyhow::bail!(
        "fail to send request to all currency API: {}",
        error_trace.join("\n\n")
    )
}
