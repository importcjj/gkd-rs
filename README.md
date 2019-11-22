# gkd-rs
A multi-connection TCP accelerator, written in Rust

为TCP加速

## 原理及概念

#### Connection

这里及下文中所提到的Connection并未一般意义上的TCP连接，在我们的项目里，它是一个抽象出来的东西。Connection虽然可以读写，但是其并不是直接与服务器通信。Connection拥有多个channel。客户端的Connection在处理上游请求时，会先把从上游读取的请求data封装成带有自增ID和Connection ID的packet，然后通过send channel发送给Tunnel Manager，我们预先建立的Tunnel会通过消耗channel的方式获取这些packet，并通过自己的TCP连接将它们发送给服务端。服务端的Tunnel Manager收到这些packet后，会根据每个packet的Connection ID分发给对应的存在于服务端的Connection，Connection在处理接收到的时候需要借助缓存调整packet的顺序，这样能够保证发送给目标地址的数据是顺序有效的。

A. 客户端

1. 客户端会有自己的Peer ID
2. 客户端启动时会与服务端建立起多个Tunnel, 即真实的TCP连接。
3. 处理上游的请求时，新建Connection，并发送一个不带任何数据的connect packet。接着就是边读取上游data，边封packet交由Tunnel Manager发送给服务端。需要注意的是，服务端返回的数据packet是乱序的，Connection在返回给上游请求方时需要借助内部缓存来调整顺序，这样上游方收到的数据才是正确的。

B. 服务端

> 服务端应该支持服务多个Peer，Peer之间彼此隔离，互不影响

以下处理步骤仅针对一个Peer来讲

1. Tunnel Manager从N个Tunnel的recv channel中读取packet，然后根据packet携带的connection ID取出相应的Connection，如不存在则新建。将packet交由Connection处理。若该packet为connect类型，则读取包中含有的dest server地址并与其建立TCP连接。Connection从Tunnel Manager得到的packet很大概率是乱序的，所以需要借助内部缓存来调整顺序，从而保证发送给dest server的数据是正常的原始数据。相对的，Connection从dest server读取数据时，可以边read，边封packet，然后直接交由Tunnel Manager发送给相应的客户端。
