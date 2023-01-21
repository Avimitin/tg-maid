use crate::modules::{cache::*, provider::*};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub(crate) struct Runtime<CACHE, R>
where
    CACHE: CurrenciesCache + CollectedMsgCache + KsyxCounterCache,
    R: Send
        + Sync
        + NsfwProvider
        + ArchLinuxPkgProvider
        + WeatherProvider
        + RecipeProvider
        + EhentaiProvider
        + CurrenciesRateProvider,
{
    pub cache: Arc<Mutex<CACHE>>,
    pub req: R,
    pub translator: Arc<deepl::DeepLApi>,
}

/// Default implementation use memory to store
impl<T, R> Runtime<T, R>
where
    T: CurrenciesCache + CollectedMsgCache + KsyxCounterCache,
    R: Send
        + Sync
        + NsfwProvider
        + ArchLinuxPkgProvider
        + WeatherProvider
        + RecipeProvider
        + EhentaiProvider
        + CurrenciesRateProvider,
{
    pub fn new(cache: T, req: R, tr: deepl::DeepLApi) -> Self {
        Self {
            req,
            translator: Arc::new(tr),
            cache: Arc::new(Mutex::new(cache)),
        }
    }
}
