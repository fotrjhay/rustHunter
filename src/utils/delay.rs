use tokio::time::{sleep, Duration};

pub async fn human_delay() {
    sleep(Duration::from_secs(5)).await;
}
