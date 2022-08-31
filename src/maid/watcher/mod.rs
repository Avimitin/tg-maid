#[cfg(feature = "weibo")]
pub mod weibo;

#[cfg(feature = "osu")]
pub mod osu;

pub mod bili;

async fn notify_on_ctrl_c(tx: tokio::sync::watch::Sender<u8>) {
    tokio::signal::ctrl_c().await.ok();
    tracing::info!("quiting osu watcher...");
    tx.send(0).expect("fail to send signal into osu watcher");
}
