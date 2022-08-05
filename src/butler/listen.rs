use warp::Filter;

/// Spawn a health check listener in a non-blocking task for Docker HEALTHCHECK task
pub fn spawn_healthcheck_listner(port: u16) {
    // FIXME: This is an orphan task!
    tokio::task::spawn(async move {
        let heartbeat = warp::path("/healthcheck").map(|| "bong bong");
        warp::serve(heartbeat).run(([127, 0, 0, 1], port)).await;
    });
}
