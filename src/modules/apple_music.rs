//! Apple Music URL matching, preview handling, and public concert-page extraction.
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Context, Result};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde_json::Value;
use teloxide::types::InputFile;
use tokio::sync::Mutex;

use super::{
    apple_music_album::{AlbumLink, AlbumPreviews},
    Sendable,
};

/// Apple-specific link variants stay behind the URL reply coordinator.
pub(super) enum AppleUrl {
    Concert(ConcertLink),
    Album(AlbumLink),
}

impl AppleUrl {
    fn parse(url: &Url) -> Option<Self> {
        ConcertLink::parse(url)
            .map(Self::Concert)
            .or_else(|| AlbumLink::parse(url).map(Self::Album))
    }

    pub(super) fn key(&self) -> String {
        match self {
            Self::Concert(link) => format!("concert:{}", link.event_id),
            Self::Album(link) => link.key(),
        }
    }
}

pub(super) fn apple_url_matcher(original: &str, cleaned: &Url) -> Option<AppleUrl> {
    // Apple's i= parameter selects a track. Preserve it even if a generic URL
    // cleaner strips it; otherwise recognize Apple URLs resolved by cleanup.
    Url::parse(original)
        .ok()
        .as_ref()
        .and_then(AppleUrl::parse)
        .or_else(|| AppleUrl::parse(cleaned))
}

/// Owns Apple Music clients/caches and turns native links into generic replies.
pub struct AppleMusic {
    concerts: ConcertPreviews,
    albums: AlbumPreviews,
}

impl AppleMusic {
    pub fn new(proxy: Option<&str>) -> Result<Self> {
        Ok(Self {
            concerts: ConcertPreviews::new(proxy)?,
            albums: AlbumPreviews::new(proxy)?,
        })
    }

    pub(super) async fn handle_url(&self, link: AppleUrl) -> Result<Sendable> {
        let (png, caption, filename) = match link {
            AppleUrl::Concert(link) => {
                let concert = self.concerts.get(&link).await?;
                let caption = concert.caption_html(&link.url);
                let png =
                    tokio::task::spawn_blocking(move || super::concert_card::render(&concert))
                        .await??;
                (png, caption, "concert.png")
            }
            AppleUrl::Album(link) => {
                let album = self.albums.get(&link).await?;
                let caption = album.caption_html();
                let png = tokio::task::spawn_blocking(move || super::album_card::render(&album))
                    .await??;
                (png, caption, "album.png")
            }
        };
        Ok(Sendable::File(
            InputFile::memory(png).file_name(filename),
            Some(caption),
        ))
    }
}

const CACHE_TTL: Duration = Duration::from_secs(30 * 60);
const CACHE_LIMIT: usize = 256;
const MAX_PAGE_BYTES: usize = 3 * 1024 * 1024;
const MAX_ARTWORK_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct ConcertLink {
    pub event_id: String,
    pub url: Url,
}

impl ConcertLink {
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
        let id = match parts.as_slice() {
            ["", region, "concerts", id]
                if region.len() == 2 && region.bytes().all(|b| b.is_ascii_alphabetic()) =>
            {
                *id
            }
            ["", "concerts", id] => *id,
            _ => return None,
        };
        let uuid = id.strip_prefix("ce.")?;
        if uuid.len() != 36
            || !uuid.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_hexdigit()
                }
            })
        {
            return None;
        }
        let mut canonical = url.clone();
        canonical.set_scheme("https").ok()?;
        canonical.set_query(None);
        canonical.set_fragment(None);
        Some(Self {
            event_id: id.to_ascii_lowercase(),
            url: canonical,
        })
    }

    fn matches(&self, value: &str) -> bool {
        Url::parse(value)
            .ok()
            .and_then(|url| Self::parse(&url))
            .is_some_and(|other| other.event_id == self.event_id)
    }
}

#[derive(Clone, Debug)]
pub struct Artist {
    pub name: String,
    pub url: Option<Url>,
}

#[derive(Clone, Debug)]
pub struct Concert {
    pub title: String,
    pub artists: Vec<Artist>,
    pub listed_time: String,
    pub venue: String,
    pub address: Option<String>,
    pub tickets: Option<Url>,
    pub share_artwork_url: Option<Url>,
    pub share_artwork: Option<Arc<[u8]>>,
}

impl Concert {
    pub fn caption_html(&self, url: &Url) -> String {
        use teloxide::utils::html::escape;
        let artists = self
            .artists
            .iter()
            .take(3)
            .map(|artist| {
                let name = bounded(&artist.name, 80);
                match &artist.url {
                    Some(url) => {
                        format!("<a href=\"{}\">{}</a>", escape(url.as_str()), escape(&name))
                    }
                    None => escape(&name),
                }
            })
            .collect::<Vec<_>>()
            .join(" · ");
        let mut links = Vec::new();
        if let Some(tickets) = &self.tickets {
            links.push(format!(
                "<a href=\"{}\">Book tickets</a>",
                escape(tickets.as_str())
            ));
        }
        links.push(format!(
            "<a href=\"{}\">Apple Music</a>",
            escape(url.as_str())
        ));
        if artists.is_empty() {
            links.join(" · ")
        } else {
            format!("{artists}\n{}", links.join(" · "))
        }
    }
}

pub struct ConcertPreviews {
    client: Client,
    cache: Mutex<HashMap<String, (Instant, Concert)>>,
}

impl ConcertPreviews {
    pub fn new(proxy: Option<&str>) -> Result<Self> {
        let mut builder = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .user_agent("tg-maid/0.1 (concert previews)")
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                let original = attempt.previous().first().and_then(ConcertLink::parse);
                if attempt.previous().len() <= 3
                    && (original.is_some_and(|link| link.matches(attempt.url().as_str()))
                        || (attempt.previous().first().is_some_and(is_artwork_url)
                            && is_artwork_url(attempt.url())))
                {
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

    pub async fn get(&self, link: &ConcertLink) -> Result<Concert> {
        {
            let mut cache = self.cache.lock().await;
            cache.retain(|_, (created, _)| created.elapsed() < CACHE_TTL);
            if let Some((_, concert)) = cache.get(&link.event_id) {
                return Ok(concert.clone());
            }
        }

        let mut urls = vec![link.url.clone()];
        let fallback = Url::parse(&format!(
            "https://music.apple.com/us/concerts/{}",
            link.event_id
        ))?;
        if fallback != link.url {
            urls.push(fallback);
        }
        let mut error = anyhow!("No concert page available");
        for mut url in urls {
            url.query_pairs_mut().append_pair("l", "en-US");
            match self.fetch(url, link).await {
                Ok(mut concert) => {
                    if let Some(url) = &concert.share_artwork_url {
                        match self.fetch_artwork(url).await {
                            Ok(bytes) => concert.share_artwork = Some(bytes.into()),
                            Err(err) => {
                                tracing::debug!(error = %err, "Concert artwork unavailable")
                            }
                        }
                    }
                    let mut cache = self.cache.lock().await;
                    if cache.len() >= CACHE_LIMIT {
                        if let Some(oldest) = cache
                            .iter()
                            .min_by_key(|(_, (time, _))| time)
                            .map(|(id, _)| id.clone())
                        {
                            cache.remove(&oldest);
                        }
                    }
                    cache.insert(link.event_id.clone(), (Instant::now(), concert.clone()));
                    return Ok(concert);
                }
                Err(err) => error = err,
            }
        }
        Err(error.context("Unable to retrieve Apple Music concert"))
    }

    async fn fetch(&self, url: Url, link: &ConcertLink) -> Result<Concert> {
        let mut response = self.client.get(url).send().await?.error_for_status()?;
        ensure!(
            response.status().is_success() && link.matches(response.url().as_str()),
            "Apple Music redirected away from the concert"
        );
        ensure!(
            response.content_length().unwrap_or(0) <= MAX_PAGE_BYTES as u64,
            "Concert page exceeds size limit"
        );
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                body.len() + chunk.len() <= MAX_PAGE_BYTES,
                "Concert page exceeds size limit"
            );
            body.extend_from_slice(&chunk);
        }
        let link = link.clone();
        tokio::task::spawn_blocking(move || parse_page(&String::from_utf8_lossy(&body), &link))
            .await?
    }

    async fn fetch_artwork(&self, url: &Url) -> Result<Vec<u8>> {
        ensure!(is_artwork_url(url), "Unsupported artwork host");
        let mut response = self
            .client
            .get(url.clone())
            .send()
            .await?
            .error_for_status()?;
        ensure!(
            response.status().is_success() && is_artwork_url(response.url()),
            "Artwork redirect rejected"
        );
        ensure!(
            response.content_length().unwrap_or(0) <= MAX_ARTWORK_BYTES as u64,
            "Artwork exceeds size limit"
        );
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                bytes.len() + chunk.len() <= MAX_ARTWORK_BYTES,
                "Artwork exceeds size limit"
            );
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}

fn bounded(value: &str, max: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
}

fn text(value: &Value) -> Option<String> {
    let value = bounded(value.as_str()?, 256);
    (!value.is_empty()).then_some(value)
}

fn find_value<'a>(value: &'a Value, predicate: &impl Fn(&Value) -> bool) -> Option<&'a Value> {
    if predicate(value) {
        return Some(value);
    }
    match value {
        Value::Object(values) => values.values().find_map(|v| find_value(v, predicate)),
        Value::Array(values) => values.iter().find_map(|v| find_value(v, predicate)),
        _ => None,
    }
}

fn parse_page(html: &str, link: &ConcertLink) -> Result<Concert> {
    let document = Html::parse_document(html);
    let selector =
        Selector::parse("script[type='application/ld+json'], script#serialized-server-data")
            .unwrap();
    let values: Vec<Value> = document
        .select(&selector)
        // Read script text directly: HTML serialization would escape '&' in
        // ticket URLs before the caption's own HTML escaping step.
        .filter_map(|script| serde_json::from_str(&script.text().collect::<String>()).ok())
        .collect();
    let event = values.iter().find_map(|value| {
        find_value(value, &|v| {
            (v["@type"] == "MusicEvent"
                || v["@type"]
                    .as_array()
                    .is_some_and(|types| types.iter().any(|t| t == "MusicEvent")))
                && v["url"].as_str().is_some_and(|url| link.matches(url))
        })
    });
    // These sections are emitted by Apple's concert detail page. Keep them scoped
    // to its canonical URL; nearby/recommended concerts must never supply fields.
    let page = values.iter().find_map(|value| {
        find_value(value, &|v| {
            v["canonicalURL"]
                .as_str()
                .is_some_and(|url| link.matches(url))
                && v["sections"].is_array()
        })
    });
    let mut concert = event.and_then(|event| parse_event(event).ok());
    if let Some(page) = page {
        if let Ok(details) = parse_sections(page) {
            if let Some(concert) = &mut concert {
                concert.listed_time = details.listed_time;
                if concert.address.is_none() {
                    concert.address = details.address;
                }
                if concert.artists.is_empty() {
                    concert.artists = details.artists;
                } else {
                    for artist in &mut concert.artists {
                        if artist.url.is_none() {
                            artist.url = details
                                .artists
                                .iter()
                                .find(|other| other.name == artist.name)
                                .and_then(|other| other.url.clone());
                        }
                    }
                }
                if concert.tickets.is_none() {
                    concert.tickets = details.tickets;
                }
            } else {
                concert = Some(details);
            }
        }
        if let Some(concert) = &mut concert {
            if concert.tickets.is_none() {
                concert.tickets = page["sections"]
                    .as_array()
                    .and_then(|sections| parse_tickets(sections));
            }
        }
    }
    let mut concert =
        concert.context("No matching concert with date, time and venue in Apple Music page")?;
    // Apple's social artwork already includes the circular artist photo and
    // calendar badge. Prefer the wide share image over the square OG variant.
    for selector in [
        "meta[name='twitter:image'], meta[property='twitter:image']",
        "meta[property='og:image']",
    ] {
        let selector = Selector::parse(selector).unwrap();
        concert.share_artwork_url = document
            .select(&selector)
            .filter_map(|meta| Url::parse(meta.value().attr("content")?).ok())
            .find(is_artwork_url);
        if concert.share_artwork_url.is_some() {
            break;
        }
    }
    Ok(concert)
}

fn parse_event(event: &Value) -> Result<Concert> {
    let start = text(&event["startDate"]).context("Missing concert date")?;
    // Never assume the storefront's timezone or treat Apple's time as doors/start.
    let date = chrono::DateTime::parse_from_rfc3339(&start)?;
    let listed_time = date.format("%Y-%m-%d %H:%M (UTC%:z)").to_string();
    let location = &event["location"];
    let address = &location["address"];
    let address = text(address).or_else(|| {
        let parts: Vec<_> = [
            "streetAddress",
            "addressLocality",
            "addressRegion",
            "postalCode",
            "addressCountry",
        ]
        .iter()
        .filter_map(|key| text(&address[*key]))
        .collect();
        (!parts.is_empty()).then(|| bounded(&parts.join(", "), 384))
    });
    let performer = &event["performer"];
    let performers = performer
        .as_array()
        .cloned()
        .unwrap_or_else(|| vec![performer.clone()]);
    let artists = performers
        .iter()
        .filter_map(|p| {
            Some(Artist {
                name: text(&p["name"])?,
                url: web_url(&p["url"]),
            })
        })
        .take(8)
        .collect();
    Ok(Concert {
        title: text(&event["name"]).context("Missing event name")?,
        artists,
        listed_time,
        venue: text(&location["name"]).context("Missing venue")?,
        address,
        tickets: find_value(&event["offers"], &|v| web_url(&v["url"]).is_some())
            .and_then(|v| web_url(&v["url"])),
        share_artwork_url: None,
        share_artwork: None,
    })
}

fn parse_sections(page: &Value) -> Result<Concert> {
    let sections = page["sections"].as_array().context("Missing sections")?;
    let header = sections
        .iter()
        .find(|s| s["itemKind"] == "concertDetailHeaderLockup")
        .and_then(|s| s["items"].as_array()?.first())
        .context("Missing concert header")?;
    let list = sections
        .iter()
        .find(|s| s["itemKind"] == "concertDetailList")
        .and_then(|s| s["items"][0]["items"].as_array())
        .context("Missing concert details")?;
    let date = list
        .iter()
        .find(|item| item["segue"]["$kind"] == "addToCalendarAction")
        .context("Missing concert date")?;
    let venue = list
        .iter()
        .find(|item| item["symbolArtwork"]["name"] == "mappin.and.ellipse")
        .context("Missing concert venue")?;
    let day = text(&date["title"]).context("Missing concert day")?;
    let time = text(&date["subtitle"]).context("Missing concert time")?;
    let zone = text(&header["accessoryCalendarArtwork"]["timeZone"])
        .context("Missing concert timezone")?;
    let heading = text(&header["titleLink"]["title"]).context("Missing concert title")?;
    let descriptor = find_value(&header["titleLink"]["segue"], &|v| {
        // Analytics also contains {kind: "artist"}; only a content descriptor
        // has a catalog identifier or a usable artist URL.
        v["kind"] == "artist"
            && (v["identifiers"]["storeAdamID"].is_string() || web_url(&v["url"]).is_some())
    });
    let artists = descriptor
        .map(|descriptor| {
            vec![Artist {
                name: heading.clone(),
                url: web_url(&descriptor["url"]).or_else(|| {
                    let id = descriptor["identifiers"]["storeAdamID"].as_str()?;
                    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
                        return None;
                    }
                    Url::parse(&format!("https://music.apple.com/us/artist/{id}")).ok()
                }),
            }]
        })
        .unwrap_or_default();
    Ok(Concert {
        title: text(&date["segue"]["eventName"]).unwrap_or(heading),
        artists,
        listed_time: bounded(&format!("{day}, {time} ({zone})"), 256),
        venue: text(&venue["title"]).context("Missing venue name")?,
        address: text(&date["segue"]["address"]).or_else(|| text(&venue["subtitle"])),
        tickets: parse_tickets(sections),
        share_artwork_url: None,
        share_artwork: None,
    })
}

fn parse_tickets(sections: &[Value]) -> Option<Url> {
    let action = sections
        .iter()
        .find(|s| s["itemKind"] == "concertDetailButtonSection")?
        .pointer("/items/0/leadingButton/link/segue")?;
    find_value(action, &|v| {
        v["$kind"] == "openExternalURLAction" && web_url(&v["url"]).is_some()
    })
    .and_then(|v| web_url(&v["url"]))
}

fn web_url(value: &Value) -> Option<Url> {
    let value = value.as_str()?;
    if value.len() > 2048 {
        return None;
    }
    let url = Url::parse(value).ok()?;
    (matches!(url.scheme(), "https" | "http")
        && url.has_host()
        && url.username().is_empty()
        && url.password().is_none())
    .then_some(url)
}

pub(super) fn is_artwork_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| host.ends_with(".mzstatic.com"))
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const URL: &str = "https://music.apple.com/jp/concerts/ce.1b03d66d-296c-49f0-b2ec-7984ecc4d085";
    const FIXTURE: &str = include_str!("fixtures/apple_music_concert.html");

    fn link() -> ConcertLink {
        ConcertLink::parse(&Url::parse(URL).unwrap()).unwrap()
    }

    #[test]
    fn apple_url_matcher_preserves_album_selection_and_language() {
        let cleaned = Url::parse("https://music.apple.com/jp/album/example/123").unwrap();
        let selected = format!("{cleaned}?i=456&l=en-US&utm_source=test");
        let AppleUrl::Album(album) = apple_url_matcher(&selected, &cleaned).unwrap() else {
            panic!("Expected album");
        };
        assert_eq!(album.track_id.as_deref(), Some("456"));
        assert_eq!(album.url.query(), Some("i=456&l=en-US"));
    }

    #[test]
    fn album_keys_distinguish_tracks_but_not_storefront_aliases() {
        let parse = |url: &str| AppleUrl::parse(&Url::parse(url).unwrap()).unwrap().key();
        let track = parse("https://music.apple.com/jp/album/example/123?i=456");
        assert_ne!(
            track,
            parse("https://music.apple.com/jp/album/example/123?i=789")
        );
        assert_ne!(track, parse("https://music.apple.com/jp/album/example/123"));
        assert_eq!(
            track,
            parse("https://music.apple.com/us/album/alias/123?i=456&l=en-US")
        );
    }

    async fn cached_apple_music() -> AppleMusic {
        let apple_music = AppleMusic::new(Some("http://127.0.0.1:0")).unwrap();
        apple_music.concerts.cache.lock().await.insert(
            link().event_id,
            (Instant::now(), parse_page(FIXTURE, &link()).unwrap()),
        );
        apple_music
    }

    #[tokio::test]
    async fn url_reply_uses_real_handler_for_unchanged_failed_and_resolved_cleanup() {
        use super::super::url_preview::prepare_url_reply;
        let apple_music = cached_apple_music().await;
        for (original, cleaned) in [
            (URL, Some(Url::parse(URL).unwrap())),
            (URL, None),
            (
                "https://example.org/redirect",
                Some(Url::parse(URL).unwrap()),
            ),
        ] {
            let reply =
                prepare_url_reply(original, cleaned, &mut Default::default(), &apple_music).await;
            assert!(
                matches!(reply, Some(Sendable::File(_, Some(caption))) if caption.contains("Apple Music"))
            );
        }
    }

    #[tokio::test]
    async fn url_reply_deduplicates_only_within_a_message() {
        use super::super::url_preview::prepare_url_reply;
        let apple_music = cached_apple_music().await;
        let mut seen = Default::default();
        assert!(matches!(
            prepare_url_reply(URL, None, &mut seen, &apple_music).await,
            Some(Sendable::File(_, _))
        ));
        let alias = URL.replace("/jp/", "/us/");
        assert!(prepare_url_reply(&alias, None, &mut seen, &apple_music)
            .await
            .is_none());
        assert!(matches!(
            prepare_url_reply(&alias, None, &mut Default::default(), &apple_music).await,
            Some(Sendable::File(_, _))
        ));
    }

    #[test]
    fn concert_urls_require_exact_host_and_event_path() {
        assert!(ConcertLink::parse(
            &Url::parse(&format!("{URL}?utm_source=test#details")).unwrap()
        )
        .is_some());
        assert!(link().matches(&URL.replace("/jp/", "/us/")));
        assert!(link().matches(&URL.replace("/jp/", "/")));
        for url in [
            URL.replace("music.apple.com", "music.apple.com.example.org"),
            URL.replace("music.apple.com", "user@music.apple.com"),
            URL.replace("music.apple.com", "music.apple.com:8080"),
            URL.replace("/concerts/", "/artist/"),
            format!("{URL}/extra"),
            URL.replace("ce.", "ce.invalid"),
        ] {
            assert!(
                ConcertLink::parse(&Url::parse(&url).unwrap()).is_none(),
                "{url}"
            );
        }
    }

    #[test]
    fn extracts_local_time_artist_links_and_ticket_action() {
        let concert = parse_page(FIXTURE, &link()).unwrap();
        assert_eq!(concert.venue, "WWW X");
        assert_eq!(
            concert.listed_time,
            "December 26, 2026 · Saturday, 5:00 PM (Asia/Tokyo)"
        );
        assert!(concert.address.as_ref().unwrap().contains("宇田川町13-17"));
        assert_eq!(concert.artists[0].name, "ひとひら");
        assert_eq!(
            concert.artists[0].url.as_ref().unwrap().as_str(),
            "https://music.apple.com/jp/artist/123456789"
        );
        assert_eq!(
            concert.tickets.as_ref().unwrap().as_str(),
            "https://eplus.jp/ana_vol4/"
        );
        let caption = concert.caption_html(&link().url);
        assert_eq!(caption.lines().count(), 2);
        assert!(caption.contains(">Book tickets</a>"));
        assert!(!caption.contains("Listed time"));
    }

    #[test]
    fn serialized_sections_work_without_json_ld() {
        let fixture = FIXTURE.replace("application/ld+json", "text/ignored");
        let concert = parse_page(&fixture, &link()).unwrap();
        assert_eq!(concert.venue, "WWW X");
        assert_eq!(
            concert.artists[0].url.as_ref().unwrap().as_str(),
            "https://music.apple.com/us/artist/123456789"
        );
    }

    #[test]
    fn fills_missing_artist_url_and_keeps_tickets_when_local_time_is_absent() {
        let fixture = FIXTURE.replace("https://music.apple.com/jp/artist/123456789", "");
        let concert = parse_page(&fixture, &link()).unwrap();
        assert_eq!(
            concert.artists[0].url.as_ref().unwrap().as_str(),
            "https://music.apple.com/us/artist/123456789"
        );
        let fixture = FIXTURE.replace("Asia/Tokyo", "");
        let concert = parse_page(&fixture, &link()).unwrap();
        assert!(concert.tickets.is_some());
        assert_eq!(concert.listed_time, "2026-12-26 08:00 (UTC+00:00)");
    }

    #[test]
    fn schema_only_preserves_offset_and_never_guesses_doors_or_start() {
        let fixture = FIXTURE
            .replace("id=\"serialized-server-data\"", "id=\"ignored\"")
            .replace("2026-12-26T08:00:00Z", "2026-12-26T17:00:00+09:00");
        let concert = parse_page(&fixture, &link()).unwrap();
        assert_eq!(concert.listed_time, "2026-12-26 17:00 (UTC+09:00)");
        assert!(concert.tickets.is_none());
        assert!(!concert.caption_html(&link().url).contains("Book tickets"));
    }

    #[test]
    fn unrelated_homepage_wrong_event_and_missing_fields_fail_closed() {
        assert!(parse_page("<h1>Apple Music</h1><script type='application/ld+json'>{\"@type\":\"Organization\"}</script>", &link()).is_err());
        let wrong = FIXTURE.replace("1b03d66d", "2b03d66d");
        assert!(parse_page(&wrong, &link()).is_err());
        assert!(parse_page(&FIXTURE.replace("WWW X", ""), &link()).is_err());
        assert!(parse_page(
            &FIXTURE
                .replace("startDate", "unknown")
                .replace("addToCalendarAction", "unknown"),
            &link()
        )
        .is_err());
    }

    #[test]
    fn caption_escapes_names_and_rejects_non_web_links() {
        let mut concert = parse_page(FIXTURE, &link()).unwrap();
        concert.artists[0].name = "<b>Artist & guest</b>".into();
        let caption = concert.caption_html(&link().url);
        assert!(caption.contains("&lt;b&gt;Artist &amp; guest&lt;/b&gt;"));
        for value in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "https://user:pass@example.com/",
        ] {
            assert!(web_url(&json!(value)).is_none());
        }
    }

    #[test]
    fn ticket_query_parameters_are_not_html_encoded_during_json_parsing() {
        let ticket = "https://click.linksynergy.com/deeplink?id=example&mid=36672&murl=http://ticket.pia.jp/pia/event.do?eventCd=2625283";
        let html = FIXTURE.replace("https://eplus.jp/ana_vol4/", ticket);
        let concert = parse_page(&html, &link()).unwrap();
        assert_eq!(concert.tickets.as_ref().unwrap().as_str(), ticket);
        assert!(concert
            .tickets
            .as_ref()
            .unwrap()
            .query_pairs()
            .any(|(key, value)| key == "mid" && value == "36672"));
        let caption = concert.caption_html(&link().url);
        assert!(caption.contains("&amp;mid=36672"));
        assert!(!caption.contains("&amp;amp;"));
    }

    #[test]
    fn share_artwork_prefers_wide_apple_image_and_rejects_other_hosts() {
        let metadata = r#"<meta property="og:image" content="https://is1-ssl.mzstatic.com/square.jpg">
            <meta name="twitter:image" content="https://is1-ssl.mzstatic.com/wide.jpg">"#;
        let html = FIXTURE.replace("<head>", &format!("<head>{metadata}"));
        assert_eq!(
            parse_page(&html, &link())
                .unwrap()
                .share_artwork_url
                .unwrap()
                .path(),
            "/wide.jpg"
        );
        let html = html.replace(
            "https://is1-ssl.mzstatic.com/wide.jpg",
            "https://example.org/wide.jpg",
        );
        assert_eq!(
            parse_page(&html, &link())
                .unwrap()
                .share_artwork_url
                .unwrap()
                .path(),
            "/square.jpg"
        );
        for url in [
            "http://is1-ssl.mzstatic.com/a.jpg",
            "https://mzstatic.com.example.org/a.jpg",
            "https://127.0.0.1/a.jpg",
        ] {
            assert!(!is_artwork_url(&Url::parse(url).unwrap()));
        }
    }

    #[test]
    fn unreadable_artwork_still_renders_event_details() {
        let mut concert = parse_page(FIXTURE, &link()).unwrap();
        concert.share_artwork = Some(Arc::from(&b"not an image"[..]));
        let png = crate::modules::concert_card::render(&concert).unwrap();
        assert_eq!(image::load_from_memory(&png).unwrap().width(), 1200);
    }

    #[tokio::test]
    async fn cache_reuses_event_across_storefronts() {
        let previews = ConcertPreviews::new(None).unwrap();
        previews.cache.lock().await.insert(
            link().event_id,
            (Instant::now(), parse_page(FIXTURE, &link()).unwrap()),
        );
        let us = ConcertLink::parse(&Url::parse(&URL.replace("/jp/", "/us/")).unwrap()).unwrap();
        assert_eq!(previews.get(&us).await.unwrap().venue, "WWW X");
        assert_eq!(previews.get(&link()).await.unwrap().venue, "WWW X");
    }

    #[test]
    fn renders_japanese_card_as_png() {
        let mut concert = parse_page(FIXTURE, &link()).unwrap();
        if let Ok(path) = std::env::var("CONCERT_PREVIEW_TEST_ARTWORK") {
            concert.share_artwork = Some(std::fs::read(path).unwrap().into());
        }
        let font =
            rusttype::Font::try_from_bytes(include_bytes!(env!("QUOTE_TEXT_FONT_PATH"))).unwrap();
        for ch in "ひとひら感激記録渋谷".chars() {
            assert_ne!(font.glyph(ch).id().0, 0);
        }
        let png = crate::modules::concert_card::render(&concert).unwrap();
        let image = image::load_from_memory(&png).unwrap();
        assert_eq!(image.width(), 1200);
        assert!((630..=1600).contains(&image.height()));
        // Optional review artifact, without making normal tests write into the repo.
        if let Ok(path) = std::env::var("CONCERT_PREVIEW_TEST_OUTPUT") {
            std::fs::write(path, png).unwrap();
        }
    }
}
