//! Album and selected-track previews from Apple's public album page.
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Context, Result};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde_json::Value;
use tokio::sync::Mutex;

use super::apple_music::is_artwork_url;

#[derive(Clone, Debug)]
pub struct AlbumLink {
    pub album_id: String,
    pub track_id: Option<String>,
    pub url: Url,
}

impl AlbumLink {
    pub fn parse(url: &Url) -> Option<Self> {
        if !matches!(url.scheme(), "https" | "http")
            || url.host_str() != Some("music.apple.com")
            || url.port().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return None;
        }
        let parts: Vec<_> = url.path().trim_end_matches('/').split('/').collect();
        let (region, id) = match parts.as_slice() {
            ["", region, "album", _, id] | ["", region, "album", id] => (*region, *id),
            _ => return None,
        };
        if region.len() != 2 || !region.bytes().all(|b| b.is_ascii_alphabetic()) || !valid_id(id) {
            return None;
        }
        let tracks: Vec<_> = url
            .query_pairs()
            .filter(|(k, _)| k == "i")
            .map(|(_, v)| v.into_owned())
            .collect();
        if tracks.len() > 1 || tracks.first().is_some_and(|id| !valid_id(id)) {
            return None;
        }
        let track_id = tracks.into_iter().next();
        let language = url
            .query_pairs()
            .find(|(k, _)| k == "l")
            .map(|(_, v)| v.into_owned());
        let mut canonical = url.clone();
        canonical.set_scheme("https").ok()?;
        canonical.set_fragment(None);
        canonical.set_query(None);
        if let Some(id) = &track_id {
            canonical.query_pairs_mut().append_pair("i", id);
        }
        if let Some(language) = language {
            canonical.query_pairs_mut().append_pair("l", &language);
        }
        Some(Self {
            album_id: id.into(),
            track_id,
            url: canonical,
        })
    }

    pub fn key(&self) -> String {
        format!(
            "album:{}:{}",
            self.album_id,
            self.track_id.as_deref().unwrap_or("")
        )
    }

    fn matches(&self, url: &Url) -> bool {
        Self::parse(url).is_some_and(|other| self.album_id == other.album_id)
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit())
}

#[derive(Clone, Debug)]
pub struct Album {
    pub title: String,
    pub track_title: Option<String>,
    pub artist: String,
    pub release_date: Option<String>,
    pub track_count: Option<u64>,
    pub genre: Option<String>,
    pub url: Url,
    pub artwork_url: Option<Url>,
    pub artwork: Option<Arc<[u8]>>,
}

impl Album {
    pub fn caption_html(&self) -> String {
        format!(
            "<a href=\"{}\">Listen on Apple Music</a>",
            teloxide::utils::html::escape(self.url.as_str())
        )
    }
}

pub struct AlbumPreviews {
    client: Client,
    cache: Mutex<HashMap<String, (Instant, Album)>>,
}

impl AlbumPreviews {
    pub fn new(proxy: Option<&str>) -> Result<Self> {
        let mut builder = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .user_agent("tg-maid/0.1 (album previews)")
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                let first = attempt.previous().first();
                let same_album = first
                    .and_then(AlbumLink::parse)
                    .is_some_and(|link| link.matches(attempt.url()));
                let artwork = first.is_some_and(is_artwork_url) && is_artwork_url(attempt.url());
                if attempt.previous().len() <= 3 && (same_album || artwork) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }));
        if let Some(proxy) = proxy {
            builder = builder.proxy(reqwest::Proxy::all(proxy)?);
        }
        Ok(Self {
            client: builder.build()?,
            cache: Mutex::new(HashMap::new()),
        })
    }

    pub async fn get(&self, link: &AlbumLink) -> Result<Album> {
        // Keep localized titles separate while sharing aliases of the same album/track.
        let region = link
            .url
            .path_segments()
            .and_then(|mut p| p.next())
            .unwrap_or("");
        let language = link
            .url
            .query_pairs()
            .find(|(k, _)| k == "l")
            .map(|(_, v)| v.into_owned())
            .unwrap_or_else(|| "en-US".into());
        let key = format!("{}:{region}:{language}", link.key());
        {
            let mut cache = self.cache.lock().await;
            cache.retain(|_, (created, _)| created.elapsed() < Duration::from_secs(1800));
            if let Some((_, album)) = cache.get(&key) {
                return Ok(album.clone());
            }
        }
        let mut urls = vec![link.url.clone()];
        if region != "us" {
            let mut fallback = link.url.clone();
            fallback.set_path(&format!("/us/album/{}", link.album_id));
            urls.push(fallback);
        }
        let mut error = anyhow!("No album page available");
        for mut url in urls {
            if !url.query_pairs().any(|(k, _)| k == "l") {
                url.query_pairs_mut().append_pair("l", "en-US");
            }
            match self.fetch(url, link).await {
                Ok(mut album) => {
                    if let Some(url) = &album.artwork_url {
                        match self.download(url.clone(), 2 * 1024 * 1024, None).await {
                            Ok(bytes) => album.artwork = Some(bytes.into()),
                            Err(err) => tracing::debug!(error = %err, "Album artwork unavailable"),
                        }
                    }
                    let mut cache = self.cache.lock().await;
                    if cache.len() >= 64 {
                        if let Some(oldest) = cache
                            .iter()
                            .min_by_key(|(_, (time, _))| time)
                            .map(|(key, _)| key.clone())
                        {
                            cache.remove(&oldest);
                        }
                    }
                    cache.insert(key, (Instant::now(), album.clone()));
                    return Ok(album);
                }
                Err(err) => error = err,
            }
        }
        Err(error.context("Unable to retrieve Apple Music album"))
    }

    async fn download(&self, url: Url, limit: usize, album: Option<&AlbumLink>) -> Result<Vec<u8>> {
        ensure!(
            album.is_some() || is_artwork_url(&url),
            "Unsupported artwork host"
        );
        let mut response = self.client.get(url).send().await?.error_for_status()?;
        ensure!(
            response.status().is_success(),
            "Apple Music redirect rejected"
        );
        ensure!(
            match album {
                Some(link) => link.matches(response.url()),
                None => is_artwork_url(response.url()),
            },
            "Unexpected preview destination"
        );
        ensure!(
            response.content_length().unwrap_or(0) <= limit as u64,
            "Preview exceeds size limit"
        );
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                body.len() + chunk.len() <= limit,
                "Preview exceeds size limit"
            );
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }

    async fn fetch(&self, url: Url, link: &AlbumLink) -> Result<Album> {
        let body = self.download(url, 3 * 1024 * 1024, Some(link)).await?;
        let link = link.clone();
        tokio::task::spawn_blocking(move || parse_page(&String::from_utf8_lossy(&body), &link))
            .await?
    }
}

fn text(value: &Value) -> Option<String> {
    let text = value
        .as_str()?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!text.is_empty()).then(|| text.chars().take(500).collect())
}

fn descriptor_id(value: &Value) -> Option<&str> {
    value
        .pointer("/contentDescriptor/identifiers/storeAdamID")
        .and_then(Value::as_str)
}

fn artists(item: &Value) -> Option<String> {
    let names: Vec<_> = item["subtitleLinks"]
        .as_array()?
        .iter()
        .filter_map(|v| text(&v["title"]))
        .take(8)
        .collect();
    (!names.is_empty()).then(|| names.join(", "))
}

fn parse_page(html: &str, link: &AlbumLink) -> Result<Album> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("script#serialized-server-data").unwrap();
    let script = document
        .select(&selector)
        .next()
        .context("Album data missing")?;
    let data: Value = serde_json::from_str(&script.text().collect::<String>())?;
    // Scope metadata to the requested album, never recommendations or analytics.
    let page = data["data"]
        .as_array()
        .context("Album pages missing")?
        .iter()
        .map(|v| &v["data"])
        .find(|page| {
            page["canonicalURL"]
                .as_str()
                .and_then(|v| Url::parse(v).ok())
                .is_some_and(|url| link.matches(&url))
        })
        .context("Apple Music returned a different album")?;
    let sections = page["sections"]
        .as_array()
        .context("Album sections missing")?;
    let header = sections
        .iter()
        .filter(|s| s["itemKind"] == "containerDetailHeaderLockup")
        .filter_map(|s| s["items"].as_array())
        .flatten()
        .find(|item| {
            descriptor_id(item) == Some(link.album_id.as_str())
                && item["contentDescriptor"]["kind"] == "album"
        })
        .context("Album header missing")?;
    let selected = if let Some(track_id) = &link.track_id {
        Some(
            sections
                .iter()
                .filter(|s| s["itemKind"] == "trackLockup")
                .filter_map(|s| s["items"].as_array())
                .flatten()
                .find(|item| {
                    descriptor_id(item) == Some(track_id.as_str())
                        && item["contentDescriptor"]["kind"] == "song"
                })
                .context("Selected track is not present in this album")?,
        )
    } else {
        None
    };
    let footer = sections
        .iter()
        .filter(|s| s["itemKind"] == "containerDetailTracklistFooterLockup")
        .filter_map(|s| s["items"].as_array())
        .flatten()
        .find(|item| descriptor_id(item) == Some(link.album_id.as_str()));
    let release_date = footer
        .and_then(|f| f["description"].as_str())
        .and_then(|s| s.lines().next())
        .filter(|s| {
            ["%B %e, %Y", "%b %e, %Y", "%Y-%m-%d", "%Y年%m月%d日"]
                .iter()
                .any(|format| chrono::NaiveDate::parse_from_str(s.trim(), format).is_ok())
        })
        .and_then(|s| text(&Value::String(s.into())));
    let genre = text(&header["quaternaryTitle"]).and_then(|s| {
        let value = s.split('·').next()?.trim();
        (!value.is_empty() && !value.chars().all(|c| c.is_ascii_digit())).then(|| value.to_string())
    });
    let artwork_url = header
        .pointer("/artwork/dictionary/url")
        .and_then(Value::as_str)
        .map(|url| {
            url.replace("{w}", "800")
                .replace("{h}", "800")
                .replace("{f}", "jpg")
        })
        .and_then(|url| Url::parse(&url).ok())
        .filter(is_artwork_url);
    let mut album_url = header
        .pointer("/contentDescriptor/url")
        .and_then(Value::as_str)
        .and_then(|value| Url::parse(value).ok())
        .filter(|url| link.matches(url))
        .and_then(|url| AlbumLink::parse(&url))
        .or_else(|| {
            page["canonicalURL"]
                .as_str()
                .and_then(|value| Url::parse(value).ok())
                .and_then(|url| AlbumLink::parse(&url))
        })
        .context("Album URL missing")?
        .url;
    let language: Vec<_> = album_url
        .query_pairs()
        .filter(|(key, _)| key == "l")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    album_url.set_query(None);
    if !language.is_empty() {
        album_url.query_pairs_mut().extend_pairs(language);
    }
    Ok(Album {
        title: text(&header["title"]).context("Album title missing")?,
        track_title: selected
            .map(|item| text(&item["title"]).context("Track title missing"))
            .transpose()?,
        artist: selected
            .and_then(|item| text(&item["artistName"]).or_else(|| artists(item)))
            .or_else(|| artists(header))
            .context("Album artist missing")?,
        release_date,
        track_count: header["trackCount"].as_u64().filter(|n| *n > 0),
        genre,
        url: album_url,
        artwork_url,
        artwork: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const URL: &str =
        "https://music.apple.com/jp/album/fantastic-gathering/6828257360?i=6828257363&l=en-US";
    const FIXTURE: &str = include_str!("fixtures/apple_music_album.html");

    fn link() -> AlbumLink {
        AlbumLink::parse(&Url::parse(URL).unwrap()).unwrap()
    }

    #[test]
    fn preserves_track_and_language_but_removes_tracking() {
        let link =
            AlbumLink::parse(&Url::parse(&format!("{URL}&utm_source=test#play")).unwrap()).unwrap();
        assert_eq!(link.track_id.as_deref(), Some("6828257363"));
        assert_eq!(link.url.as_str(), URL);
        for url in [
            "https://music.apple.com.evil.org/jp/album/title/123",
            "https://user:pass@music.apple.com/jp/album/title/123",
            "https://music.apple.com:8443/jp/album/title/123",
            "https://music.apple.com/jp/album/title/not-a-number",
            "https://music.apple.com/jp/album/title/123?i=bad",
            "https://music.apple.com/jp/album/title/123?i=1&i=2",
            "https://music.apple.com/jp/playlist/title/123",
        ] {
            assert!(
                AlbumLink::parse(&Url::parse(url).unwrap()).is_none(),
                "{url}"
            );
        }
    }

    #[test]
    fn extracts_selected_song_album_metadata_and_square_artwork() {
        let album = parse_page(FIXTURE, &link()).unwrap();
        assert_eq!(album.track_title.as_deref(), Some("Fantastic Gathering!"));
        assert_eq!(album.title, "Fantastic Gathering! - Single");
        assert_eq!(album.artist, "Hatsuboshi Gakuen, Kotone Fujita & Sena Juo");
        assert_eq!(album.release_date.as_deref(), Some("November 9, 2026"));
        assert_eq!(album.track_count, Some(2));
        assert_eq!(album.genre.as_deref(), Some("Anime"));
        assert!(album
            .artwork_url
            .as_ref()
            .unwrap()
            .path()
            .ends_with("/800x800bb.jpg"));
        assert!(!album.url.query_pairs().any(|(k, _)| k == "i"));
        assert!(album
            .caption_html()
            .contains("fantastic-gathering-single/6828257360"));
    }

    #[test]
    fn different_tracks_and_album_only_have_distinct_identity() {
        let mut other = link();
        other.track_id = Some("6828257365".into());
        let instrumental = parse_page(FIXTURE, &other).unwrap();
        assert_eq!(
            instrumental.track_title.as_deref(),
            Some("Fantastic Gathering! (Instrumental)")
        );
        assert_ne!(link().key(), other.key());
        other.track_id = None;
        let album = parse_page(FIXTURE, &other).unwrap();
        assert!(album.track_title.is_none());
        assert_eq!(album.artist, "Hatsuboshi Gakuen, Kotone Fujita, Sena Juo");
        assert_ne!(link().key(), other.key());
    }

    #[test]
    fn rejects_unrelated_pages_and_missing_selected_tracks() {
        assert!(parse_page("<html>Home</html>", &link()).is_err());
        assert!(parse_page(&FIXTURE.replace("6828257360", "9999999999"), &link()).is_err());
        let mut unknown = link();
        unknown.track_id = Some("999999".into());
        assert!(parse_page(FIXTURE, &unknown).is_err());
    }

    #[test]
    fn optional_fields_are_not_invented_and_artwork_hosts_are_checked() {
        let stripped = FIXTURE
            .replace("November 9, 2026\\n", "")
            .replace("\"trackCount\": 2", "\"trackCount\": null")
            .replace("Anime · 2026", "2026")
            .replace("is1-ssl.mzstatic.com", "example.org");
        let album = parse_page(&stripped, &link()).unwrap();
        assert!(album.release_date.is_none());
        assert!(album.track_count.is_none());
        assert!(album.genre.is_none());
        assert!(album.artwork_url.is_none());
    }

    #[test]
    fn renders_bounded_png_with_artwork_and_with_invalid_artwork() {
        let mut album = parse_page(FIXTURE, &link()).unwrap();
        let cover = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([(x * 4) as u8, (y * 4) as u8, 160])
        });
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(cover)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        album.artwork = Some(bytes.into_inner().into());
        let png = crate::modules::album_card::render(&album).unwrap();
        let image = image::load_from_memory(&png).unwrap().to_rgb8();
        assert_eq!(image.dimensions(), (1200, 630));
        assert_ne!(image.get_pixel(0, 0), image.get_pixel(1199, 629));
        album.artwork = Some(Arc::from(&b"invalid image"[..]));
        album.track_title = Some("長い曲名と日本語の文字 ".repeat(30));
        album.artist = "Artist & Collaborator ".repeat(20);
        let png = crate::modules::album_card::render(&album).unwrap();
        assert_eq!(image::load_from_memory(&png).unwrap().width(), 1200);
    }
}
