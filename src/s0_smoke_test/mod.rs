use bytes::BytesMut;
use std::io;
use tokio::io::AsyncWriteExt;
use tokio::{io::AsyncReadExt, net::TcpStream};

pub async fn process(mut stream: TcpStream) -> io::Result<usize> {
    // The `Connection` lets us read/write redis **frames** instead of
    // byte streams. The `Connection` type is defined by mini-redis.
    let mut buf = BytesMut::with_capacity(100);

    while let Ok(len) = stream.read_buf(&mut buf).await {
        if len > 0 {
            println!("GOT {:?}\n", buf);
            let _ = stream.write_buf(&mut buf).await;

        } else {
            break;
        }
    }

    Ok(1)
}
