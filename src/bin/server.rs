use async_std::task;
use gkd::server::Server;

fn main() {
    let server = Server::new();
    task::block_on(server.run_server("127.0.0.1:9990"));
}
