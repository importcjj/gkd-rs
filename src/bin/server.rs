use gkd::server::Server;
use gkd::Result;

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();
    println!("Listening on :9990");
    let server = Server::bind("0.0.0.0:9990").await?;
    while let Some(conn) = server.accept().await {
        
    }
    Ok(())
}
