use async_trait::async_trait;

/// Error type that is returned by the `Server` trait
#[derive(Debug)]
pub enum ServerErrorKind {
    BindFail,
    NotBound,
    ReadFail,
    WriteFail,
}

#[async_trait]
pub trait Server
where
    Self: Default
{
    /// Method that binds a server to the address:port given
    async fn bind(&mut self, addr: &str) -> Result<(), ServerErrorKind>;

    /// Method that puts the server in listening mode that
    /// takes in new connections, reads requests and responds
    /// to them accordingly
    async fn listen(&mut self) -> Result<(), ServerErrorKind>;
}
