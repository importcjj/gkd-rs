use gkd::server::Server;
use gkd::Result;

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();
    let server = Server::new();
    println!("Listening on :9990");
    server.run_server("0.0.0.0:9990").await?;
    Ok(())
}
