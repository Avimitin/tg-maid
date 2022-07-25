use crate::modules::{collect, req, types, ksyx};
use std::sync::Arc;
// FIXME: This mutex locker is not suggested to be used
// But vaultwarden used it in their projects.
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub(crate) struct Runtime<CS, C, K>
where
    CS: types::CurrenciesStorage,
    C: collect::Collector,
    K: ksyx::KsyxCounter,
{
    pub currency_cache: Arc<Mutex<CS>>,
    pub collector: Arc<Mutex<C>>,
    pub ksyx_hit_counter: Arc<Mutex<K>>,
    pub req: Arc<req::Client>,
}

/// Default implementation use memory to store
impl<CS, C, K> Runtime<CS, C, K>
where
    CS: types::CurrenciesStorage,
    C: collect::Collector,
    K: ksyx::KsyxCounter,
{
    pub fn new(currency_cache: CS, collector: C, k_counter: K) -> Self {
        Self {
            currency_cache: Arc::new(Mutex::new(currency_cache)),
            collector: Arc::new(Mutex::new(collector)),
            req: Arc::new(req::Client::new()),
            ksyx_hit_counter: Arc::new(Mutex::new(k_counter)),
        }
    }
}
