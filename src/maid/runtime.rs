use crate::modules::{cache::*, provider::*};
use std::sync::Arc;
// FIXME: This mutex locker is not suggested to be used
// But vaultwarden used it in their projects.
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
    pub patterns: Arc<super::MsgPatternMatcher>,
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
    pub fn new(cache: T, req: R) -> Self {
        Self {
            req,
            patterns: Arc::new(super::MsgPatternMatcher::generate()),
            cache: Arc::new(Mutex::new(cache)),
        }
    }
}
