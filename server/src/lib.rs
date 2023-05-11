use async_trait::async_trait;

/// Error type that is returned by the `Server` trait
#[derive(Debug)]
pub enum ServerErrorKind {
    BindFail,
    ConnectFail,
    ReadFail,
    WriteFail,
}

#[async_trait]
pub trait Server
where
    Self: Default
{
    /// Method that starts the server
    async fn run(&mut self, addr: &str) -> Result<(), ServerErrorKind>;
}
