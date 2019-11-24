use gkd::server::Server;
use async_std::task;

fn main() {
    let server = Server::new();
    task::block_on(server.run_server("127.0.0.1:9990"));
}