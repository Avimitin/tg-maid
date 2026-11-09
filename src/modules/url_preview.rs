//! URL reply policy, independent of Telegram delivery and Apple resource types.
use std::collections::HashSet;

use reqwest::Url;

use super::{
    apple_music::{apple_url_matcher, AppleMusic},
    Sendable,
};

/// Called after URL cleanup. A preview can still be needed when cleanup made no
/// changes or failed; provider failures fall back to changed URLs only.
pub async fn prepare_url_reply(
    original: &str,
    cleaned: Option<Url>,
    seen_previews: &mut HashSet<String>,
    apple_music: &AppleMusic,
) -> Option<Sendable> {
    let changed = cleaned.as_ref().is_some_and(|url| url.as_str() != original);
    let url = cleaned.or_else(|| Url::parse(original).ok())?;
    if let Some(link) = apple_url_matcher(original, &url) {
        let key = link.key();
        if !seen_previews.insert(key.clone()) {
            return None;
        }
        match apple_music.handle_url(link).await {
            Ok(reply) => return Some(reply),
            Err(err) => {
                tracing::warn!(preview_id = %key, error = %err, "Apple Music preview unavailable")
            }
        }
    }
    changed.then(|| Sendable::Text(format!("Clean URL\n{url}")))
}

#[cfg(test)]
mod url_preview_tests {
    use super::*;

    #[tokio::test]
    async fn unrelated_urls_only_reply_when_changed() {
        let apple_music = AppleMusic::new(None).unwrap();
        for (original, changed) in [
            ("https://example.org/?utm_source=test", true),
            ("https://example.org/", false),
        ] {
            let reply = prepare_url_reply(
                original,
                Some(Url::parse("https://example.org/").unwrap()),
                &mut HashSet::new(),
                &apple_music,
            )
            .await;
            if changed {
                assert!(
                    matches!(reply, Some(Sendable::Text(text)) if text == "Clean URL\nhttps://example.org/")
                );
            } else {
                assert!(reply.is_none());
            }
        }
        assert!(
            prepare_url_reply("not a URL", None, &mut HashSet::new(), &apple_music)
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn apple_fetch_failure_preserves_cleanup_policy() {
        // Port zero refuses connections locally, exercising the real handler's
        // error path without contacting Apple or replacing it with a callback.
        let apple_music = AppleMusic::new(Some("http://127.0.0.1:0")).unwrap();
        for url in [
            "https://music.apple.com/jp/album/example/123",
            "https://music.apple.com/jp/concerts/ce.1b03d66d-296c-49f0-b2ec-7984ecc4d085",
        ] {
            for (original, changed) in [
                (url.to_owned(), false),
                (format!("{url}?utm_source=test"), true),
            ] {
                let reply = prepare_url_reply(
                    &original,
                    Some(Url::parse(url).unwrap()),
                    &mut HashSet::new(),
                    &apple_music,
                )
                .await;
                assert_eq!(matches!(reply, Some(Sendable::Text(_))), changed);
            }
        }
    }
}
