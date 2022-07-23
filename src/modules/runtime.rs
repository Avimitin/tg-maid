use crate::modules::{collect, currency, weather, req};
use std::sync::Arc;
// FIXME: This mutex locker is not suggested to be used
// But vaultwarden used it in their projects.
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub(crate) struct Runtime<CS, C, WF>
where
    CS: currency::CurrenciesStorage,
    WF: weather::WeatherFetcher,
    C: collect::Collector,
{
    pub currency: Arc<Mutex<currency::RateCalculator<CS>>>,
    pub weather: Arc<WF>,
    pub collector: Arc<Mutex<C>>,
    pub req: Arc<req::Client>
}

/// Default implementation use memory to store
impl<CS, C, WF> Runtime<CS, C, WF>
where
    CS: currency::CurrenciesStorage + Send + Sync + Clone,
    WF: weather::WeatherFetcher + Send + Sync + Clone,
    C: collect::Collector + Send + Sync + Clone,
{
    pub fn new(currency_cache: CS, collector: C, weather_cache: WF) -> Self {
        Self {
            currency: Arc::new(Mutex::new(currency::RateCalculator::new(currency_cache))),
            collector: Arc::new(Mutex::new(collector)),
            weather: Arc::new(weather_cache),
            req: Arc::new(req::Client::new()),
        }
    }
}
