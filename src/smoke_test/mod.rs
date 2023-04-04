
use bytes::BytesMut;
use std::io;
use tokio::io::AsyncWriteExt;
use tokio::{net::TcpStream, io::AsyncReadExt};


pub async fn process(mut socket: TcpStream) -> io::Result<usize> {
    // The `Connection` lets us read/write redis **frames** instead of
    // byte streams. The `Connection` type is defined by mini-redis.
    let mut buf = BytesMut::with_capacity(100);
    
    while let Ok(len) = socket.read_buf(&mut buf).await {
        if len > 0 {
            println!("GOT {:?}\n", buf);
            socket.write_buf(&mut buf).await;
        }
        else {
            break;
        }
    }

    Ok(1)
}