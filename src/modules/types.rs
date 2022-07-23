use serde::Deserialize;

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
