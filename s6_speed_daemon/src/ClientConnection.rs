
use async_trait::async_trait;
use crate::{InternalMessage, ServerErrorKind};

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufStream};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{mpsc, Mutex};

const MSG_HEADER_LEN: usize = 1;
const MSG_TYPE_ERROR: u8 = 0x10;
const MSG_TYPE_PLATE: u8 = 0x20;
const MSG_TYPE_TICKET: u8 = 0x21;
const MSG_TYPE_WANT_HEARTBEAT: u8 = 0x40;
const MSG_TYPE_HEARTBEAT: u8 = 0x41;
const MSG_TYPE_I_AM_CAMERA: u8 = 0x80;
const MSG_TYPE_I_AM_CAMERA_BODY_LEN: usize = 6;
const MSG_TYPE_I_AM_DISPATCHER: u8 = 0x81;
const MSG_TYPE_I_AM_DISPATCHER_ROADS_LEN: usize = 1;
const MSG_TYPE_I_AM_DISPATCHER_ROADS_SIZE: usize = 2;

#[derive(Debug)]
pub struct GenericClient {
    // clients: Arc<Mutex<HashMap<SocketAddr, ClientType>>>,
    // cameras: Arc<Mutex<HashMap<u16, ClientInfo>>>,
    // dispatchers: Arc<Mutex<HashMap<u16, ClientInfo>>>,
}

impl GenericClient {
    pub fn new(
        // clients: Arc<Mutex<HashMap<SocketAddr, ClientType>>>,
        // cameras: Arc<Mutex<HashMap<u16, ClientInfo>>>,
        // dispatchers: Arc<Mutex<HashMap<u16, ClientInfo>>>,
        ) -> Self {
        Self {
            // clients,
            // cameras,
            // dispatchers,
        }
    }

    async fn process(
        self,
        addr: SocketAddr,
        stream: TcpStream,
        ) -> Result<(), ServerErrorKind> {
        let mut stream = BufStream::new(stream);

        let msg_type_byte = get_u8(&mut stream).await?;

        match msg_type_byte {
            MSG_TYPE_I_AM_CAMERA => {
                dbg!("Camera");

                let road = get_u16(&mut stream).await?;
                let mile = get_u16(&mut stream).await?;
                let limit = get_u16(&mut stream).await?;

                let mut client = CameraClient {
                    stream,
                    road,
                    mile,
                    limit
                };
                dbg!(&client);

                client.run(addr).await?;

                Ok(())
            }
            MSG_TYPE_I_AM_DISPATCHER => {
                dbg!("Dispatcher");

                let numroads = get_u8(&mut stream).await?;
                let mut roads = vec![0u8; MSG_TYPE_I_AM_DISPATCHER_ROADS_SIZE*numroads as usize];

                /* Read roads */
                let read_len = stream
                    .read_exact(&mut roads)
                    .await
                    .map_err(|_| ServerErrorKind::ReadFail)?;

                if read_len != MSG_TYPE_I_AM_DISPATCHER_ROADS_SIZE*numroads as usize {
                    return Err(ServerErrorKind::ReadFail);
                }

                /* Parse roads */
                let mut roads_vec = vec![];
                for i in 0..numroads {
                    let road = u16::from_be_bytes(roads[(2*i) as usize..(2*i + 2) as usize].try_into().map_err(|_| ServerErrorKind::ServerParseFail)?);
                    roads_vec.push(road);
                }


                let mut client = DispatcherClient{stream, roads: roads_vec};
                dbg!(&client);

                client.run(addr).await?;

                Ok(())
            }
            _ => {
                // send error to client
                let response = String::from("Unknown message type").as_bytes().to_vec();
                stream
                    .write_all(&response)
                    .await
                    .map_err(|_| ServerErrorKind::WriteFail)?;

                // Flush the buffer to ensure it is sent
                stream
                    .flush()
                    .await
                    .map_err(|_| ServerErrorKind::WriteFail)?;

                Err(ServerErrorKind::UnknownClient)
            }
        
        }
    }

    pub async fn run(
        self,
        consumer: Arc<Mutex<UnboundedSender<InternalMessage>>>,
        addr: SocketAddr,
        stream: TcpStream,
        ) -> Result<(), ServerErrorKind> {

        let client = self.process(addr, stream).await?;

        Ok(())
    }
}



#[derive(Debug)]
struct CameraClient {
    road: u16,
    mile: u16,
    limit: u16,
    stream: BufStream<TcpStream>,
    // clients: Arc<Mutex<HashMap<SocketAddr, ClientType>>>,
    // cameras: Arc<Mutex<HashMap<u16, ClientInfo>>>,
    // dispatchers: Arc<Mutex<HashMap<u16, ClientInfo>>>,
}

impl CameraClient {
    async fn run(
        &mut self,
        // consumer: Arc<Mutex<UnboundedSender<InternalMessage>>>,
        // messages: Arc<Mutex<VecDeque<MessageType>>>,
        addr: SocketAddr,
    ) -> Result<(), ServerErrorKind> {
        let mut msg_type_byte = [0u8; MSG_HEADER_LEN];
        let mut should_continue = true;

        let (tx, mut rx) = mpsc::unbounded_channel::<InternalMessage>();

        // Send a message to the consumer
        // consumer
        //     .lock()
        //     .await
        //     .send(InternalMessage::NewClient)
        //     .unwrap();

        while should_continue {
            // Internal message handling
            tokio::select! {
                // If there is a message from a peer
                Some(msg) = rx.recv() => {
                    let response = match msg {
                        InternalMessage::NewClient => {
                            vec![]
                        }
                        _ => {
                            vec![]
                        }
                    };

                    if !response.is_empty() {
                        // Send the message to the other end
                        self.stream
                            .write_all(&response)
                            .await
                            .map_err(|_| ServerErrorKind::WriteFail)?;

                        // Flush the buffer to ensure it is sent
                        self.stream
                            .flush()
                            .await
                            .map_err(|_| ServerErrorKind::WriteFail)?;
                    }
                }
                // External message handling
                result = self.stream.read_exact(&mut msg_type_byte) => {
                    let read_len = result.map_err(|_| ServerErrorKind::ReadFail)?;

                    // Treat Camera messages
                    match msg_type_byte[0] {
                        MSG_TYPE_PLATE => {
                            dbg!("Plate");

                            let plate = get_str(&mut self.stream).await?;
                            let timestamp = get_u32(&mut self.stream).await?;

                            dbg!(plate, timestamp);

                            should_continue = true;
                        }
                        MSG_TYPE_WANT_HEARTBEAT => {
                            dbg!("Want Heartbeat");

                            let interval = get_u16(&mut self.stream).await?;

                            dbg!(interval);
                            should_continue = true;
                        }
                        _ => {

                            // send error to client
                            let response = String::from("Unknown message type").as_bytes().to_vec();
                            self.stream
                                .write_all(&response)
                                .await
                                .map_err(|_| ServerErrorKind::WriteFail)?;

                            // Flush the buffer to ensure it is sent
                            self.stream
                                .flush()
                                .await
                                .map_err(|_| ServerErrorKind::WriteFail)?;

                            should_continue = false;
                        }
                    }

                }
            }
        }

        dbg!("Connection closed: {}", addr);

        Ok(())
    }
}


#[derive(Debug)]
struct DispatcherClient {
    stream: BufStream<TcpStream>, 
    roads: Vec<u16>,
    // clients: Arc<Mutex<HashMap<SocketAddr, ClientType>>>,
    // cameras: Arc<Mutex<HashMap<u16, ClientInfo>>>,
    // dispatchers: Arc<Mutex<HashMap<u16, ClientInfo>>>,
}

impl DispatcherClient {
    async fn run(
        &mut self,
        // consumer: Arc<Mutex<UnboundedSender<InternalMessage>>>,
        // messages: Arc<Mutex<VecDeque<MessageType>>>,
        addr: SocketAddr,
        ) -> Result<(), ServerErrorKind> {
        let mut msg_type_byte = [0u8; MSG_HEADER_LEN];
        let mut should_continue = true;

        let (tx, mut rx) = mpsc::unbounded_channel::<InternalMessage>();

        // Send a message to the consumer
        // consumer
        //     .lock()
        //     .await
        //     .send(InternalMessage::NewClient)
        //     .unwrap();

        while should_continue {
            // Internal message handling
            tokio::select! {
                // If there is a message from a peer
                Some(msg) = rx.recv() => {
                    let response = match msg {
                        InternalMessage::NewClient => {
                            vec![]
                        }
                        _ => {
                            vec![]
                        }
                    };

                    if !response.is_empty() {
                        // Send the message to the other end
                        self.stream
                            .write_all(&response)
                            .await
                            .map_err(|_| ServerErrorKind::WriteFail)?;

                        // Flush the buffer to ensure it is sent
                        self.stream
                            .flush()
                            .await
                            .map_err(|_| ServerErrorKind::WriteFail)?;
                    }
                }
                // External message handling
                result = self.stream.read_exact(&mut msg_type_byte) => {
                    let read_len = result.map_err(|_| ServerErrorKind::ReadFail)?;

                    // Treat Dispatcher messages
                    match msg_type_byte[0] {
                        MSG_TYPE_WANT_HEARTBEAT => {
                            should_continue = true;
                        }
                        _ => {
                            // send error to client
                            let response = String::from("Unknown message type").as_bytes().to_vec();
                            self
                                .stream
                                .write_all(&response)
                                .await
                                .map_err(|_| ServerErrorKind::WriteFail)?;

                            // Flush the buffer to ensure it is sent
                            self.stream
                                .flush()
                                .await
                                .map_err(|_| ServerErrorKind::WriteFail)?;

                            should_continue = false;
                        }
                    }

                }
            }
        }

        Ok(())
    }
}

async fn get_u8(stream: &mut BufStream<TcpStream>) -> Result<u8, ServerErrorKind> {
    let mut buffer = [0u8; 1];

    let read_len = stream
        .read_exact(&mut buffer)
        .await
        .map_err(|_| ServerErrorKind::ReadFail)?;

    if read_len != 1 {
        return Err(ServerErrorKind::ReadFail);
    }

    Ok(buffer[0])
}

async fn get_u16(stream: &mut BufStream<TcpStream>) -> Result<u16, ServerErrorKind> {
    let mut buffer = [0u8; 2];

    let read_len = stream
        .read_exact(&mut buffer)
        .await
        .map_err(|_| ServerErrorKind::ReadFail)?;

    if read_len != 2 {
        return Err(ServerErrorKind::ReadFail);
    }

    Ok(u16::from_be_bytes(buffer[0..2].try_into().map_err(|_| ServerErrorKind::ServerParseFail)?))
}

async fn get_u32(stream: &mut BufStream<TcpStream>) -> Result<u32, ServerErrorKind> {
    let mut buffer = [0u8; 4];

    let read_len = stream
        .read_exact(&mut buffer)
        .await
        .map_err(|_| ServerErrorKind::ReadFail)?;

    if read_len != 4 {
        return Err(ServerErrorKind::ReadFail);
    }

    Ok(u32::from_be_bytes(buffer[0..4].try_into().map_err(|_| ServerErrorKind::ServerParseFail)?))
}

async fn get_str(stream: &mut BufStream<TcpStream>) -> Result<String, ServerErrorKind> {
    let str_len = get_u8(stream).await? as usize;
    dbg!(str_len);

    if str_len >= 1 {
        let mut buffer = vec![0u8; str_len];

        let read_len = stream
            .read_exact(&mut buffer)
            .await
            .map_err(|_| ServerErrorKind::ReadFail)?;

        if read_len != str_len {
            return Err(ServerErrorKind::ReadFail);
        }

        let s = String::from_utf8(buffer).map_err(|_| ServerErrorKind::ServerParseFail)?;

        Ok(s)
    }
    else {
        Err(ServerErrorKind::ServerParseFail)
    }
}

#[cfg(test)]
mod test {
    use super::*;

}
