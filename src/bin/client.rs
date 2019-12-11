use async_std::prelude::*;
use gkd::client::Client;
use gkd::Result;
#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();
    let client = Client::new("103.126.101.87:9990", 2).await?;
    let mut conn = client.connect("127.0.0.1:5555").await?;

    for i in 1..10u8 {
        let bytes = [i, i, i];
        conn.write_all(&bytes).await?;
    }

    // sleep(Duration::from_secs(3)).await;

    for _i in 1..10u8 {
        let mut bytes = vec![0; 3];
        conn.read_exact(&mut bytes).await?;
        println!("read [{:?}]", bytes);
    }

    Ok(())
}
