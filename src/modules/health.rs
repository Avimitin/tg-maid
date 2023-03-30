use tokio::{io::AsyncWriteExt, net::TcpListener};

/// Spawn a health check listener in a non-blocking task for Docker HEALTHCHECK task
pub fn spawn_healthcheck_listner() {
    let port = std::env::var("HEALTHCHECK_PORT")
        .unwrap_or_else(|_| "11451".to_string())
        .parse::<u16>()
        .expect("Invalid health check port number!");

    tokio::task::spawn(async move {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .expect("fail to bind docker health listener");

        tracing::info!("Docker health check listening on port {port}");

        while let Ok((mut stream, _)) = listener.accept().await {
            tracing::debug!("New Stream Incoming");
            let res = stream.write_all(b"OK").await;
            if let Err(err) = res {
                tracing::error!("fail to response to health checker: {err}")
            }
        }
    });
}

#[tokio::test]
async fn test_healthcheck() {
    spawn_healthcheck_listner();

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    use tokio::process::Command;
    let op = Command::new("nc")
        .args(["127.0.0.1", "11451"])
        .output()
        .await
        .expect("fail to execute netcat");
    assert_eq!("OK", String::from_utf8(op.stdout).unwrap());
}
