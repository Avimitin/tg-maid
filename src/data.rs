use anyhow::Result;
use async_trait::async_trait;
use reqwest::IntoUrl;
use std::ops::Deref;
use std::sync::Arc;
use teloxide::types::InputFile;
use tokio::sync::Mutex;

pub struct Data<C: CacheManager, D: DataFetcher>(Arc<RuntimeData<C, D>>);

impl<C: CacheManager, D: DataFetcher> From<RuntimeData<C, D>> for Data<C, D> {
    fn from(data: RuntimeData<C, D>) -> Self {
        Self(Arc::new(data))
    }
}

impl<C: CacheManager, D: DataFetcher> Clone for Data<C, D> {
    fn clone(&self) -> Self {
        Data(Arc::clone(&self.0))
    }
}

impl<C: CacheManager, D: DataFetcher> Deref for Data<C, D> {
    type Target = Arc<RuntimeData<C, D>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(typed_builder::TypedBuilder)]
pub struct RuntimeData<C, D> {
    pub cacher: Mutex<C>,
    pub requester: D,
}

#[async_trait]
pub trait CacheManager: Send + Sync {
    async fn collect_message(&mut self) -> Result<u32>;
    async fn finish_collect_message(&mut self) -> Option<String>;
    async fn hit_ksyx_once(&mut self) -> Result<u32>;
    async fn alias_osu_uid(&mut self, tg_id: u64, osu_id: u64) -> Result<()>;
    async fn get_osu_uid(&mut self, tg_id: u64) -> Result<u64>;
}

pub enum Sendable {
    Text(String),
    File(InputFile, Option<String>),
}

impl Sendable {
    pub fn builder() -> SendableBuilder<(), (), ()> {
        SendableBuilder {
            text: (),
            file: (),
            caption: (),
        }
    }
}

pub struct SendableBuilder<T, F, C> {
    text: T,
    file: F,
    caption: C,
}

impl SendableBuilder<(), (), ()> {
    pub fn text(self, s: impl std::fmt::Display) -> SendableBuilder<String, (), ()> {
        SendableBuilder {
            text: s.to_string(),
            file: (),
            caption: (),
        }
    }

    pub fn url(self, u: impl IntoUrl) -> SendableBuilder<(), InputFile, ()> {
        SendableBuilder {
            file: InputFile::url(u.into_url().unwrap()),
            text: (),
            caption: (),
        }
    }
}

impl SendableBuilder<String, (), ()> {
    pub fn build(self) -> Sendable {
        Sendable::Text(self.text)
    }
}

impl SendableBuilder<(), InputFile, ()> {
    pub fn build(self) -> Sendable {
        Sendable::File(self.file, None)
    }

    pub fn caption(self, c: impl std::fmt::Display) -> SendableBuilder<(), InputFile, String> {
        SendableBuilder {
            text: (),
            file: self.file,
            caption: c.to_string(),
        }
    }
}

impl SendableBuilder<(), InputFile, String> {
    pub fn build(self) -> Sendable {
        Sendable::File(self.file, Some(self.caption))
    }
}

type FetchResult = Result<Sendable>;

#[async_trait]
pub trait DataFetcher: Send + Sync {
    // NSFW Provider
    async fn fetch_nsfw_anime_img(&self) -> FetchResult;
    async fn fetch_nsfw_photo(&self) -> FetchResult;

    // Arch Linux Provider
    async fn fetch_pkg_list(&self, pkg: &str, max: usize) -> FetchResult;
    async fn fetch_pkg_info(&self, pkg: &str) -> FetchResult;

    // Weather Provider
    async fn fetch_weather(&self, city: &str) -> FetchResult;

    // Eat what? Provider
    async fn fetch_food(&self) -> FetchResult;

    // Cook Piggy Provider
    async fn fetch_pig_recipe(&self) -> FetchResult;

    // E-Hentai Information Provider
    async fn fetch_ehentai(&self, gidlist: &[[&str; 2]]) -> FetchResult;

    // Currencies
    async fn fetch_currency_rate(&self, from: &str, to: &str) -> FetchResult;
}
