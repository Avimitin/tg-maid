use serde::Deserialize;

/// Represent the konachan API response json
#[derive(Deserialize, Debug)]
pub struct KonachanApiResponse {
    pub jpeg_url: String,
    pub file_url: String,
    pub file_size: u32,
    pub author: String,
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

#[derive(Deserialize)]
pub struct ArchLinuxSearchResponse {
    valid: bool,
    results: Vec<ArchLinuxPkgInfo>,
}

impl ArchLinuxSearchResponse {
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    #[inline]
    pub fn results(&self) -> &[ArchLinuxPkgInfo] {
        &self.results
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
}

#[derive(Deserialize, Debug)]
pub struct ArchLinuxPkgInfo {
    pub pkgname: String,
    pub repo: String,
    pub pkgver: String,
    pub pkgrel: String,
    pub pkgdesc: String,
    pub url: String,
    pub installed_size: u32,
    pub last_update: String,
}

impl std::fmt::Display for ArchLinuxPkgInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Name: {}
Repo: {}
Version: {}-{}
Description: {}
Upstream: {}
Installed Size: {}
Last Update: {}
",
            self.pkgname,
            self.repo,
            self.pkgver,
            self.pkgrel,
            self.pkgdesc,
            self.url,
            self.installed_size,
            self.last_update
        )
    }
}
