use async_std::io;
use async_std::net::TcpStream;
use async_std::prelude::*;

#[async_std::main]
async fn main() -> io::Result<()> {
    let mut stream = TcpStream::connect("localhost:5555").await?;
    stream.write_all("PING".as_bytes()).await?;

    let mut rtn = vec![0; 4];
    stream.read_exact(&mut rtn).await?;

    println!("{:?}", rtn);
    Ok(())
}
