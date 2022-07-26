use crate::modules::types;
use anyhow::Context;
use rand::Rng;
use std::collections::HashMap;

lazy_static::lazy_static! {
    static ref CURRENCY_CODE_URLS: Vec<reqwest::Url> = {
        vec![
            "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies.min.json",
            "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies.json",
            "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies.min.json",
            "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies.json"
        ]
        .iter()
        .map(|url| reqwest::Url::parse(url).unwrap())
        .collect()
    };
}

/// A wrapper for re-using the reqwest client.
#[derive(Debug)]
pub struct Client {
    c: reqwest::Client,
}

impl std::default::Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self {
            c: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Fail to build req client"),
        }
    }

    /// Make a GET request to the given URL and parse the JSON response to given T type.
    /// Please make sure that the given URL will return JSON response.
    ///
    /// # Error
    /// Return error if:
    ///     * Fail to send HTTP request
    ///     * Fail to get response
    ///     * Response is not JSON
    ///     * Fail to parse response into given type
    #[inline]
    async fn to_t<T: serde::de::DeserializeOwned>(&self, url: reqwest::Url) -> anyhow::Result<T> {
        Ok(self.c.get(url).send().await?.json::<T>().await?)
    }

    #[inline]
    async fn fetch(&self, url: reqwest::Url) -> anyhow::Result<String> {
        Ok(self.c.get(url).send().await?.text().await?)
    }

    pub async fn konachan_explicit_nsfw_image(&self) -> anyhow::Result<(reqwest::Url, String)> {
        const LINK: &str = "https://konachan.com/post.json?limit=200&tags=%20rating:explicit";
        let link = reqwest::Url::parse(LINK).unwrap();

        use crate::modules::types::KonachanApiResponse;
        let response = self
            .to_t::<Vec<KonachanApiResponse>>(link)
            .await
            .with_context(|| "fail to get resp from konachan API")?;

        let mut choice = rand::thread_rng();
        let choice = choice.gen_range(0..response.len());
        let response = &response[choice];

        Ok((
            reqwest::Url::parse(&response.jpeg_url)?,
            format!(
                "<a href=\"{}\">Download Link</a>\nSize: {:.2} MB, Author: {}",
                response.file_url,
                response.file_size as f32 / 1000000.0,
                response.author
            ),
        ))
    }

    pub async fn wttr_in_weather(&self, city: &str) -> anyhow::Result<(String, String)> {
        const WTTR_IN_URL: &str = "https://wttr.in";
        let url = reqwest::Url::parse_with_params(
            &format!("{WTTR_IN_URL}/{city}"),
            &[("format", "%l的天气:+%c+温度:%t+湿度:%h+降雨量:%p")],
        )?;
        let resp = self.c.get(url).send().await?.text().await?;
        Ok((resp, format!("{WTTR_IN_URL}/{city}.png")))
    }

    pub async fn get_currency_codes(&self) -> anyhow::Result<HashMap<String, String>> {
        let mut error_trace = Vec::new();
        for url in CURRENCY_CODE_URLS.iter() {
            match self.to_t::<HashMap<String, String>>(url.clone()).await {
                Ok(codes) => {
                    return Ok(codes);
                }
                Err(e) => {
                    // TODO: Logging
                    error_trace.push(e.to_string())
                }
            }
        }

        anyhow::bail!("fail to fetch currencies: {}", error_trace.join("\n\n"))
    }

    pub async fn get_currency_rate(
        &self,
        from: &str,
        to: &str,
    ) -> anyhow::Result<types::CurrencyRateInfo> {
        macro_rules! format_array {
            ( [ $( $pattern:literal ),+ $(,)? ] ) => {
                [ $( format!($pattern ) ),+ ]
            };
        }

        // Thanks Asuna (GitHub @SpriteOvO)
        let fallbacks_urls = format_array!([
              "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies/{from}/{to}.min.json",
              "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies/{from}/{to}.json",
              "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies/{from}/{to}.min.json",
              "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies/{from}/{to}.json"
        ]);

        let mut error_trace = Vec::new();
        for url in &fallbacks_urls {
            let url = reqwest::Url::parse(url)
                .with_context(|| format!("invalid url input: {from} and {to}"))?;

            match self
                .to_t::<HashMap<String, types::CurrencyV1PossibleResponse>>(url)
                .await
            {
                Ok(res) => {
                    let rate = res
                        .get(to)
                        .ok_or_else(|| anyhow::anyhow!("fail to get response"))?
                        .unwrap_rate();
                    let date = res
                        .get("date")
                        .expect("Expect response contains date field, but got nil")
                        .unwrap_date();
                    return Ok(types::CurrencyRateInfo::new(date, rate));
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

    pub async fn get_mjx(&self) -> anyhow::Result<reqwest::Url> {
        let fallbacks_urls = [
            "https://api.uomg.com/api/rand.img3?format=json",
            "https://api.vvhan.com/api/tao?type=json",
        ];

        let mut trace = Vec::new();

        for url in fallbacks_urls {
            let url = reqwest::Url::parse(url).unwrap();

            match self.to_t::<types::MjxApiPossibleReponse>(url).await {
                Ok(res) => return Ok(reqwest::Url::parse(&res.unwrap_url())?),

                Err(e) => {
                    trace.push(e.to_string());
                }
            }
        }

        anyhow::bail!(
            "fail to make request to all TaoBao API: {}",
            trace.join("\n\n")
        )
    }

    pub async fn get_piggy_recipe(&self) -> anyhow::Result<String> {
        let page: u32 = rand::thread_rng().gen_range(0..600);
        let url = reqwest::Url::parse(&format!(
            "https://www.meishichina.com/YuanLiao/ZhuRou/{page}"
        ))
        .unwrap();

        self.fetch(url).await
    }

    pub async fn query_eh_api(
        &self,
        gid_list: &[[String; 2]],
    ) -> anyhow::Result<types::EhentaiMetadataResponse> {
        let api_url: reqwest::Url =
            reqwest::Url::parse("https://api.e-hentai.org/api.php").unwrap();

        let request_data = types::EhentaiRequestType::new(gid_list);

        self.c
            .post(api_url)
            .json(&request_data)
            .send()
            .await?
            .json::<types::PossibleEhentaiResponse>()
            .await?
            .try_unwrap()
    }
}
