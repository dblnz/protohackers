use async_trait::async_trait;
use bytes::BytesMut;
use tokio::io::AsyncWriteExt;
use tokio::{io::AsyncReadExt, net::TcpStream};

use crate::solution::{ProtoHSolution, SolutionError};

#[derive(Debug)]
pub struct SmokeTestSolution;

#[async_trait]
impl ProtoHSolution for SmokeTestSolution {
    async fn handle_stream(&mut self, socket: TcpStream) -> Result<usize, SolutionError> {
        let mut stream = socket;
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
}
