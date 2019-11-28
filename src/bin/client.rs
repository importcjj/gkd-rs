use async_std::prelude::*;
use gkd::client::Client;
use gkd::Result;

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();
    let client = Client::new("127.0.0.1:9990", 4).await?;
    let mut conn = client.connect("127.0.0.1:9998").await?;

    for i in 1..10u8 {
        let bytes = [i, i, i];
        conn.write_all(&bytes).await?;
    }

    println!("========================================");
    loop {}
    Ok(())
}
