use std::future::Future;

pub trait VideoDownloader: Send + Sized {
    fn download_from_url(u: &str) -> impl Future<Output = anyhow::Result<Self>> + Send;
    fn provide_caption(&self) -> String;
}
