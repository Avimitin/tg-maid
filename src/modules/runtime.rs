use crate::modules::{collect, ksyx, req, types};
use std::sync::Arc;
// FIXME: This mutex locker is not suggested to be used
// But vaultwarden used it in their projects.
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub(crate) struct Runtime<RC>
where
    RC: types::CurrenciesCache + collect::CollectedMsgCache + ksyx::KsyxCounterCache,
{
    pub cache: Arc<Mutex<RC>>,
    pub req: Arc<req::Client>,
}

/// Default implementation use memory to store
impl<RC> Runtime<RC>
where
    RC: types::CurrenciesCache + collect::CollectedMsgCache + ksyx::KsyxCounterCache,
{
    pub fn new(cache: RC) -> Self {
        Self {
            cache: Arc::new(Mutex::new(cache)),
            req: Arc::new(req::Client::new()),
        }
    }
}
