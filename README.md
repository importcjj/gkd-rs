# gkd-rs
A multi-connection TCP accelerator, written in Rust

为TCP加速

## 原理及概念

#### Connection

这里及下文中所提到的Connection并未一般意义上的TCP连接，在我们的项目里，它是一个抽象出来的东西。Connection虽然可以读写，但是其并不是直接与服务器通信。
Connection拥有多个channel。客户端的Connection在处理上游请求时，会先把从上游读取的请求data封装成带有自增ID和Connection ID的packet，然后通过send channel发送给
Tunnel Manager，我们预先建立的Tunnel会通过消耗channel的方式获取这些packet，并通过自己的TCP连接将它们发送给服务端。服务端的Tunnel Manager收到这
些packet后，会根据每个packet的Connection ID分发给对应的存在于服务端的Connection，Connection在处理接收到的时候需要借助缓存调整packet的顺序，这样
能够保证发送给目标地址是数据是顺序有效的。

A. 客户端

1. 客户端启动时会与服务端建立起多个Tunnel, 即真实的TCP连接.
2. 处理上游的请求时，新建Connection，并发送一个不带任何数据的connect packet.

B. 服务端
C. Connection
