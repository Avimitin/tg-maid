use crate::butler::Fetcher;
use async_trait::async_trait;

/// Implement weather report function.
#[async_trait]
pub trait WeatherProvider {
    type Format;

    /// Return formatted weather information of the given city
    async fn fetch_weather(&self, city: &str) -> Self::Format;
}

const WTTR_IN_URL: &str = "https://wttr.in";

#[async_trait]
impl WeatherProvider for Fetcher {
    type Format = anyhow::Result<(String, String)>;

    /// Implement the `WeatherReporter` trait with the wttr.in API
    async fn fetch_weather(&self, city: &str) -> Self::Format {
        let url = reqwest::Url::parse_with_params(
            &format!("{WTTR_IN_URL}/{city}"),
            &[("format", "%l的天气:+%c+温度:%t+湿度:%h+降雨量:%p")],
        )?;
        let resp = self.c.get(url).send().await?.text().await?;
        Ok((resp, format!("{WTTR_IN_URL}/{city}.png")))
    }
}
