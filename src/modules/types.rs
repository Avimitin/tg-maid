use serde::Deserialize;
use std::collections::HashMap;

/// Represent the konachan API response json
#[derive(Deserialize, Debug)]
pub struct KonachanApiResponse {
    pub jpeg_url: String,
    pub file_url: String,
    pub file_size: u32,
    pub author: String,
}

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

#[async_trait::async_trait]
pub trait CurrenciesStorage: Send + Sync + Clone {
    async fn verify_date(&mut self) -> bool;
    async fn update_currency_codes(&mut self, codes: HashMap<String, String>);
    async fn get_fullname(&mut self, code: &str) -> Option<String>;
}

/// The response from MJX API is different. This type can match those different response.
/// And its associate function can help extract the image link from response.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum MjxApiPossibleReponse {
    Uomg { code: u8, imgurl: String },
    Vvhan { title: String, pic: String },
}

impl MjxApiPossibleReponse {
    /// Extract the image url from response
    pub fn unwrap_url(self) -> String {
        match self {
            Self::Uomg { imgurl, .. } => imgurl,
            Self::Vvhan { pic, .. } => pic,
        }
    }
}
