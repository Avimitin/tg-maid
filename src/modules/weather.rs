#[async_trait::async_trait]
pub trait WeatherFetcher: Send + Sync + Clone {
    async fn query(&self, city: &str) -> anyhow::Result<String>;
}

const WTTR_IN_URL: &str = "https://wttr.in";

#[derive(Debug, Clone)]
pub struct WttrInApi {
    client: reqwest::Client,
}

impl WttrInApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub fn pic(&self, city: &str) -> String {
        format!("{WTTR_IN_URL}/{city}.png")
    }
}

#[async_trait::async_trait]
impl WeatherFetcher for WttrInApi {
    async fn query(&self, city: &str) -> anyhow::Result<String> {
        let url = reqwest::Url::parse_with_params(
            &format!("{WTTR_IN_URL}/{city}"),
            &[("format", "%l的天气:+%c+温度:%t+湿度:%h+降雨量:%p")],
        )?;
        Ok(self.client.get(url).send().await?.text().await?)
    }
}
