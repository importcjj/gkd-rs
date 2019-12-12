use async_std::io;
use async_std::task;
use gkd::Result;
use gkd::Server;

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();
    println!("Listening on :9990");
    let server = Server::bind("0.0.0.0:9990").await?;
    while let Some((conn, addr)) = server.accept().await {
        println!("serve {}", addr);
        task::spawn(async move {
            let (r, w) = &mut (&conn, &conn);
            io::copy(r, w).await.unwrap();
        });
    }
    Ok(())
}
