# gkd-rs
A multi-connection TCP accelerator, written in Rust

为TCP加速而生

## Runtime

* async-std
* tokio 0.2

## 原理及概念

简单来说，就是原来通过一条Tcp连接收发的数据包现在借助N条Tcp来进行收发，达到提速的效果。

#### Connection

这里及下文中所提到的Connection并未一般意义上的TCP连接，在我们的项目里，它是一个抽象出来的东西。Connection在客户端和服务端的行为具有差异。客户端的Connection借助多个Tunnel进行通信，这些Connection在处理上游请求时，会先把从上游读取的data封装成带有自增ID和Connection ID的packet，然后通过channel发送给Tunnel Manager，于此同时，我们预先建立的N条Tunnel会通过poll channel的方式获取这些packet，并通过自己的TCP连接发送给服务端。服务端的Tunnel Manager收到这些packet后，会根据每个packet的Connection ID分发给对应的存在于服务端的Connection，而每个Connection会重新这些调整packet的顺序。

#### Client

1. 客户端会有自己的Peer ID
2. 客户端启动时会与服务端建立起N条Tunnel, 即真实的TCP连接。
3. 处理上游的请求时，新建Connection，并发送一个不带任何数据的connect packet。接着就是边读取上游data，边封packet交由Tunnel Manager发送给服务端。需要注意的是，服务端返回的数据packet是乱序的，客户端Connection在接收server端的发来的packet时，应调整顺序，这样上游方收到的数据才是正确的。

#### Server

注: 服务端应该支持服务多个Peer，Peer通过ID区分，彼此之间隔离，互不影响

以下过程仅针对一个Peer来讲

Tunnel Manager从N个Tunnel的recv channel中读取packet，并根据packet携带的connection ID找到对应的Connection(如不存在则新建)，然后将packet交由该Connection处理。服务端的Connection处理packet时，若该packet为connect类型，则读取包中含有的dest server地址并与其建立TCP连接。Connection从Tunnel Manager得到的packet很大概率是乱序的，所以需要借助内部缓存来调整顺序，从而保证发送给dest server的数据是正常的原始数据。相对的，Connection在写数据时，直接将数据封成packet，然后直接交由Tunnel Manager发送给相应的客户端。

## Usage

echo server
```rust
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

```

client
```rust
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
```