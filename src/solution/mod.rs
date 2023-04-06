use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufStream, AsyncReadExt};

/// Custom Error type used to treat Solution specific errors
#[derive(Debug)]
pub enum SolutionError {
    General,
    Read,
    Write,
}

#[async_trait]
pub trait ProtoHSolution {
    /// Custom method to process each received request/line
    fn process_request(&mut self, line: &[u8]) -> Vec<u8>;

    /// Handles a stream that produces requests the Solution has to respond to.
    ///
    /// Returns a `Result` which contains the number of processed bytes on the
    /// success path and a custom defined error `SolutionError`
    ///
    /// This method can be customly implemented for a new solution to produce
    /// requests in a different way(e.g. multi line)
    async fn handle_stream<T>(&mut self, socket: T) -> Result<usize, SolutionError>
    where
        T: AsyncReadExt + AsyncWriteExt + Send + Sync + Unpin {

        let mut stream = BufStream::new(socket);
        let mut line = String::new();
        let mut len = 0;

        // Loop until no bytes are read
        loop {
            // Read line - Return ReadError in case of fail
            let read_len = stream
                .read_line(&mut line)
                .await
                .map_err(|_| SolutionError::Read)?;

            if read_len > 0 {

                len += read_len;

                dbg!(&line);

                // Process the received request/line
                // TODO: Prime Time says we need to close connection when malformed
                let response = self.process_request(&line.as_bytes());

                dbg!(String::from_utf8(response.clone()).unwrap());

                // Send back the result
                stream
                    .write_all(&response)
                    .await
                    .map_err(|_| SolutionError::Write)?;

                // Flush the buffer to ensure it is sent
                stream.flush().await.map_err(|_| SolutionError::Write)?;
            } else {
                break;
            }

            line.clear();
        }

        Ok(len)
    }
}
