mod client;
mod connection;
mod packet;
mod peer;
mod server;
mod tunnel;

use async_std::future::Future;
use async_std::task;

pub use client::Client;
pub use connection::Connection;
pub use server::Server;

// pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type Result<T> = async_std::io::Result<T>;

fn spawn_and_log_err<F>(fut: F) -> task::JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            log::error!("conn err: {:?}", e);
        }
    })
}
