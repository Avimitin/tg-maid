use serde::Deserialize;

/// Represent the konachan API response json
#[derive(Deserialize)]
pub struct KonachanApiResponse {
    pub jpeg_url: String,
    pub file_url: String,
    pub file_size: u32,
    pub author: String,
}
