use gkd::client::Client;
use gkd::Result;

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();
    let _client = Client::new("127.0.0.1:9990", 4).await?;
    loop {}
    Ok(())
}
