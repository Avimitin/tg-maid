use crate::modules::{collect, currency, req};
use std::sync::Arc;
// FIXME: This mutex locker is not suggested to be used
// But vaultwarden used it in their projects.
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub(crate) struct Runtime<CS, C>
where
    CS: currency::CurrenciesStorage,
    C: collect::Collector,
{
    pub currency: Arc<Mutex<currency::RateCalculator<CS>>>,
    pub collector: Arc<Mutex<C>>,
    pub req: Arc<req::Client>,
}

/// Default implementation use memory to store
impl<CS, C> Runtime<CS, C>
where
    CS: currency::CurrenciesStorage,
    C: collect::Collector,
{
    pub fn new(currency_cache: CS, collector: C) -> Self {
        Self {
            currency: Arc::new(Mutex::new(currency::RateCalculator::new(currency_cache))),
            collector: Arc::new(Mutex::new(collector)),
            req: Arc::new(req::Client::new()),
        }
    }
}
