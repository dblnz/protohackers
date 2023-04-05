use async_trait::async_trait;
use tokio::net::TcpStream;

#[derive(Debug)]
pub enum SolutionError {
    GeneralError,
}

#[async_trait]
pub trait ProtoHSolution {
    async fn handle_stream(&mut self, stream: TcpStream) -> Result<usize, SolutionError>;
}
