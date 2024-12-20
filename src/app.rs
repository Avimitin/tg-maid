use std::ops::Deref;
use std::sync::Arc;

use clearurl::UrlCleaner;
use deepl::DeepLApi;

use crate::{cache::Cacher, http::HttpClient};

pub struct AppData(Arc<RuntimeData>);

impl From<RuntimeData> for AppData {
    fn from(data: RuntimeData) -> Self {
        Self(Arc::new(data))
    }
}

impl Clone for AppData {
    fn clone(&self) -> Self {
        AppData(Arc::clone(&self.0))
    }
}

impl Deref for AppData {
    type Target = Arc<RuntimeData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(typed_builder::TypedBuilder)]
pub struct RuntimeData {
    pub cacher: Cacher,
    pub requester: HttpClient,

    pub deepl: DeepLApi,

    pub quote_maker: make_quote::QuoteProducer<'static>,

    pub url_cleaner: UrlCleaner,
}
