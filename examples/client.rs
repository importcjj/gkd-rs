use async_std::prelude::*;
use gkd::Client;
use gkd::Result;
#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();
    let tunnel_num = 4;
    let client = Client::connect("127.0.0.1:9990", tunnel_num).await?;
    let mut conn = client.get_connection().await?;

    for i in 1..10u8 {
        let bytes = [i, i, i];
        conn.write_all(&bytes).await?;
    }

    for _i in 1..10u8 {
        let mut bytes = vec![0; 3];
        conn.read_exact(&mut bytes).await?;
        println!("read [{:?}]", bytes);
    }

    Ok(())
}
