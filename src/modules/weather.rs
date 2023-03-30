use anyhow::Result;

use crate::app::AppData;

use super::Sendable;

const WTTR_IN_URL: &str = "https://wttr.in";

pub async fn fetch_weather(data: AppData, city: &str) -> Result<Sendable> {
    let url = reqwest::Url::parse_with_params(
        &format!("{WTTR_IN_URL}/{city}"),
        &[("format", "%l的天气:+%c+温度:%t+湿度:%h+降雨量:%p")],
    )?;
    let resp = data.requester.get_text(url).await?;
    Ok(Sendable::builder()
        .url(format!("{WTTR_IN_URL}/{city}.png"))
        .caption(resp)
        .build())
}
