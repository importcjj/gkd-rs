use gkd::server::Server;
use gkd::Result;

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();
    let server = Server::new();
    println!("Listening on 127.0.0.1:9990");
    server.run_server("127.0.0.1:9990").await?;
    Ok(())
}
