use std::fmt::Display;

use anyhow::Context;
use rand::{distributions::Alphanumeric, Rng};
use scraper::{Html, Selector};
use serde::Deserialize;

#[derive(Debug)]
pub struct JDPriceAnalyzer();

impl JDPriceAnalyzer {
    pub async fn get(item_id: &str) -> anyhow::Result<impl PriceInfo + std::fmt::Debug> {
        let price_endpoint: reqwest::Url =
            reqwest::Url::parse("https://gwdang.com/trend/").unwrap();

        // We need to wrap rand::thread_rng() in a block, to force it drop before await
        let (phpsessid, fp, dfp) = {
            // This website requires fingerprint for data API.
            // So although it is expensive, but for privacy reason, I still wants to use random cookie
            // for it.
            let mut rng = rand::thread_rng();
            let phpsessid: String = (0..26).map(|_| rng.sample(Alphanumeric) as char).collect();
            let fp: String = (0..33).map(|_| rng.sample(Alphanumeric) as char).collect();
            let dfp = (0..73)
                .map(|_| rng.sample(Alphanumeric) as char)
                .collect::<String>()
                .to_uppercase();

            (phpsessid, fp, dfp)
        };

        let jar = reqwest::cookie::Jar::default();
        jar.add_cookie_str(&format!("phpsessid={phpsessid}"), &price_endpoint);
        jar.add_cookie_str(&format!("fp={fp}"), &price_endpoint);
        jar.add_cookie_str(&format!("dfp={dfp}"), &price_endpoint);

        let http_client = reqwest::Client::builder()
            .cookie_store(true)
            .cookie_provider(jar.into())
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:122.0) Gecko/20100101 Firefox/122.0",
            )
            .build()
            .unwrap();

        let mut url = price_endpoint.join("data_www").unwrap();
        url.query_pairs_mut()
            .append_pair("v", "2")
            .append_pair("dp_id", &format!("{item_id}-3"))
            .finish();
        let mut info = http_client
            .get(url)
            .send()
            .await
            .with_context(|| "fail to send request for JD price")?
            .error_for_status()
            .with_context(|| "fail to get request from JD price backend")?
            .json::<GWDangJDPrice>()
            .await
            .with_context(|| "fail to deserialize result from for JD price")?;

        let page_url = price_endpoint.join(&format!("{item_id}-3.html")).unwrap();
        let page = http_client
            .get(page_url)
            .send()
            .await
            .with_context(|| "fail to send request for JD price")?
            .error_for_status()
            .with_context(|| "fail to get request from JD price backend")?
            .text()
            .await
            .with_context(|| "fail to deserialize result from for JD price")?;
        let dom = Html::parse_document(&page);
        let selector = Selector::parse(".dp-img").unwrap();
        if let Some((thumbnail, name)) = dom
            .select(&selector)
            .map(|elem| Some((elem.attr("src")?, elem.attr("alt")?)))
            .next()
            .unwrap()
        {
            info.product_name = name.to_string();
            info.product_thumbnail = Some(thumbnail.to_string());
        }

        Ok(info)
    }
}

pub struct Currency {
    pub unit: String,
    pub amount: f64,
}

impl Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.amount, self.unit)
    }
}

pub struct Price {
    pub listed: Currency,
    pub current: Currency,
    pub lowest: Currency,
}

pub trait PriceInfo {
    fn name(&self) -> impl ToString + Display;
    fn price(&self) -> Price;
    fn sales_info(&self) -> impl ToString + Display;
    fn thumbnail(&self) -> Option<String>;
}

/// JD Price info from https://gwdang.com
#[derive(Debug, Clone, Deserialize)]
pub struct GWDangJDPrice {
    #[serde(skip)]
    pub product_name: String,
    #[serde(skip)]
    pub product_thumbnail: Option<String>,

    pub product_original: GWDangProductOriginal,
    pub product_status: GWDangProductStatus,
    pub current_promo: GWDangCurrentPromo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GWDangProductOriginal {
    pub current: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GWDangProductStatus {
    pub last: u64,
    pub current: u64,
    pub status_text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GWDangCurrentPromo {
    pub promo_info: String,
}

impl PriceInfo for GWDangJDPrice {
    fn name(&self) -> impl ToString + Display {
        &self.product_name
    }

    fn price(&self) -> Price {
        let builder = |a: u64| Currency {
            unit: "CNY".to_string(),
            amount: a as f64 / 100.0,
        };

        Price {
            listed: builder(self.product_original.current),
            current: builder(self.product_status.current),
            lowest: builder(self.product_status.last),
        }
    }

    fn sales_info(&self) -> impl ToString + Display {
        &self.current_promo.promo_info
    }

    fn thumbnail(&self) -> Option<String> {
        self.product_thumbnail.clone()
    }
}

#[tokio::test]
async fn test_get_jd_price() {
    let price = JDPriceAnalyzer::get("100066293792").await.unwrap();

    dbg!(price);
}
