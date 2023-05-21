use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedSender;
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;

use server::{Server, ServerErrorKind};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufStream};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

/// Speed Daemon
///
/// Motorists on Freedom Island drive as fast as they like.
/// Sadly, this has led to a large number of crashes, so the
///  islanders have agreed to impose speed limits.
/// The speed limits will be enforced via an average speed check:
///  Automatic Number Plate Recognition cameras will be installed
///  at various points on the road network.
/// The islanders will use a computer system to collect the data,
///  detect cars travelling in excess of the speed limit,
///  and send tickets to be dispatched to the drivers.
/// The islanders can't agree on which one of them should write the software,
///  so they've engaged an external contractor to do it: that's where you come in.
///
/// Overview
///
/// You need to build a server to coordinate enforcement of average speed limits
///  on the Freedom Island road network.
///
/// Your server will handle two types of client: cameras and ticket dispatchers.
///
/// Clients connect over TCP and speak a protocol using a binary format.
/// Make sure you support at least 150 simultaneous clients.
///
/// Cameras
///
/// Each camera is on a specific road, at a specific location, and has a specific speed limit.
/// Each camera provides this information when it connects to the server.
/// Cameras report each number plate that they observe, along with the timestamp
///  that they observed it.
/// Timestamps are exactly the same as Unix timestamps (counting seconds since 1st of January 1970),
///  except that they are unsigned.
///
/// Ticket dispatchers
///
/// Each ticket dispatcher is responsible for some number of roads.
/// When the server finds that a car was detected at 2 points on the same
///  road with an average speed in excess of the speed limit (speed = distance / time),
///  it will find the responsible ticket dispatcher and send it a ticket for the offending car,
///  so that the ticket dispatcher can perform the necessary legal rituals.
///
/// Roads
///
/// Each road in the network is identified by a number from 0 to 65535.
/// A single road has the same speed limit at every point on the road.
/// Positions on the roads are identified by the number of miles from the start of the road.
/// Remarkably, all speed cameras are positioned at exact integer numbers of miles from the start of the road.
///
/// Cars
///
/// Each car has a specific number plate represented as an uppercase alphanumeric string.
///
/// Data types
///
/// The protocol uses a binary data format, with the following primitive types:
/// u8, u16, u32
///
/// These types represent unsigned integers of 8-bit, 16-bit, and 32-bit size, respectively.
/// They are transmitted in network byte-order (big endian).
///
/// Examples:
///
/// Type | Hex data    | Value
/// -------------------------------
/// u8   |          20 |         32
/// u8   |          e3 |        227
/// u16  |       00 20 |         32
/// u16  |       12 45 |       4677
/// u16  |       a8 23 |      43043
/// u32  | 00 00 00 20 |         32
/// u32  | 00 00 12 45 |       4677
/// u32  | a6 a9 b5 67 | 2796139879
///
/// str
///
/// A string of characters in a length-prefixed format.
/// A str is transmitted as a single u8 containing the string's length (0 to 255),
///  followed by that many bytes of u8, in order, containing ASCII character codes.
///
/// Examples:
///
/// Type | Hex data                   | Value
/// ----------------------------------------------
/// str  | 00                         | ""
/// str  | 03 66 6f 6f                | "foo"
/// str  | 08 45 6C 62 65 72 65 74 68 | "Elbereth"
///
/// Message types
///
/// Each message starts with a single u8 specifying the message type.
/// This is followed by the message contents, as detailed below.
///
/// Field names are not transmitted. You know which field is which by the order they are in.
///
/// There is no message delimiter. Messages are simply concatenated together with no padding.
/// The 2nd message starts with the byte that comes immediately after the final byte
///  of the 1st message, and so on.
///
/// In the examples shown below, the hexadecimal data is broken across several lines
///  to aid comprehension, but of course in the real protocol there is no such distinction.
///
/// It is an error for a client to send the server a message with any message type value
///  that is not listed below with "Client->Server".
///
/// 0x10: Error (Server->Client)
///
/// Fields:
///     msg: str
///
/// When the client does something that this protocol specification declares "an error",
///  the server must send the client an appropriate Error message and immediately disconnect that client.
///
/// Examples:
///
/// Hexadecimal:                            Decoded:
/// 10                                      Error{
/// 03 62 61 64                                 msg: "bad"
///                                         }
///
/// 10                                      Error{
/// 0b 69 6c 6c 65 67 61 6c 20 6d 73 67         msg: "illegal msg"
///                                         }
///
/// 0x20: Plate (Client->Server)
///
/// Fields:
///     plate: str
///     timestamp: u32
///
/// This client has observed the given number plate at its location, at the given timestamp.
/// Cameras can send observations in any order they like, and after any delay they like,
///  so you won't necessarily receive observations in the order that they were made.
/// This means a later Plate message may correspond to an earlier observation (with lower timestamp)
///  even if they're both from the same camera.
/// You need to take observation timestamps from the Plate message. Ignore your local system clock.
///
/// It is an error for a client that has not identified itself as a camera (see IAmCamera below)
///  to send a Plate message.
///
/// Examples:
///
/// Hexadecimal:                Decoded:
/// 20                          Plate{
/// 04 55 4e 31 58                  plate: "UN1X",
/// 00 00 03 e8                     timestamp: 1000
///                             }
///
/// 20                          Plate{
/// 07 52 45 30 35 42 4b 47         plate: "RE05BKG",
/// 00 01 e2 40                     timestamp: 123456
///                             }
///
/// 0x21: Ticket (Server->Client)
///
/// Fields:
///     plate: str
///     road: u16
///     mile1: u16
///     timestamp1: u32
///     mile2: u16
///     timestamp2: u32
///     speed: u16 (100x miles per hour)
///
/// When the server detects that a car's average speed exceeded the speed limit between
///  2 observations, it generates a Ticket message detailing the number plate of the car (plate),
///  the road number of the cameras (road), the positions of the cameras (mile1, mile2),
///  the timestamps of the observations (timestamp1, timestamp2), and the inferred average
///  speed of the car multiplied by 100, and expressed as an integer (speed).
///
/// mile1 and timestamp1 must refer to the earlier of the 2 observations (the smaller timestamp),
///  and mile2 and timestamp2 must refer to the later of the 2 observations (the larger timestamp).
///
/// The server sends the ticket to a dispatcher for the corresponding road.
///
/// Examples:
///
/// Hexadecimal:            Decoded:
/// 21                      Ticket{
/// 04 55 4e 31 58              plate: "UN1X",
/// 00 42                       road: 66,
/// 00 64                       mile1: 100,
/// 00 01 e2 40                 timestamp1: 123456,
/// 00 6e                       mile2: 110,
/// 00 01 e3 a8                 timestamp2: 123816,
/// 27 10                       speed: 10000,
///                         }
///
/// 21                      Ticket{
/// 07 52 45 30 35 42 4b 47     plate: "RE05BKG",
/// 01 70                       road: 368,
/// 04 d2                       mile1: 1234,
/// 00 0f 42 40                 timestamp1: 1000000,
/// 04 d3                       mile2: 1235,
/// 00 0f 42 7c                 timestamp2: 1000060,
/// 17 70                       speed: 6000,
///                         }
///
/// 0x40: WantHeartbeat (Client->Server)
///
/// Fields:
///     interval: u32 (deciseconds)
///
/// Request heartbeats.
///
/// The server must now send Heartbeat messages to this client at the given interval,
///  which is specified in "deciseconds", of which there are 10 per second.
/// (So an interval of "25" would mean a Heartbeat message every 2.5 seconds).
/// The heartbeats help to assure the client that the server is still functioning,
///  even in the absence of any other communication.
///
/// An interval of 0 deciseconds means the client does not want to receive heartbeats
///  (this is the default setting).
///
/// It is an error for a client to send multiple WantHeartbeat messages on a single connection.
///
/// Examples:
///
/// Hexadecimal:    Decoded:
/// 40              WantHeartbeat{
/// 00 00 00 0a         interval: 10
///                 }
///
/// 40              WantHeartbeat{
/// 00 00 04 db         interval: 1243
///                 }
///
/// 0x41: Heartbeat (Server->Client)
///
/// No fields.
///
/// Sent to a client at the interval requested by the client.
///
/// Example:
///
/// Hexadecimal:    Decoded:
/// 41              Heartbeat{}
///
/// 0x80: IAmCamera (Client->Server)
///
/// Fields:
///     road: u16
///     mile: u16
///     limit: u16 (miles per hour)
///
/// This client is a camera. The road field contains the road number that the camera is on,
///  mile contains the position of the camera, relative to the start of the road,
///  and limit contains the speed limit of the road, in miles per hour.
///
/// It is an error for a client that has already identified itself as either a camera or a ticket dispatcher to send an IAmCamera message.
///
/// Examples:
///
/// Hexadecimal:    Decoded:
/// 80              IAmCamera{
/// 00 42               road: 66,
/// 00 64               mile: 100,
/// 00 3c               limit: 60,
///                 }
///
/// 80              IAmCamera{
/// 01 70               road: 368,
/// 04 d2               mile: 1234,
/// 00 28               limit: 40,
///                 }
///
/// 0x81: IAmDispatcher (Client->Server)
///
/// Fields:
///     numroads: u8
///     roads: [u16] (array of u16)
///
/// This client is a ticket dispatcher. The numroads field says how many roads this dispatcher
///  is responsible for, and the roads field contains the road numbers.
///
/// It is an error for a client that has already identified itself as either a camera or a
///  ticket dispatcher to send an IAmDispatcher message.
///
/// Examples:
///
/// Hexadecimal:    Decoded:
/// 81              IAmDispatcher{
/// 01                  roads: [
/// 00 42                   66
///                     ]
///                 }
///
/// 81              IAmDispatcher{
/// 03                  roads: [
/// 00 42                   66,
/// 01 70                   368,
/// 13 88                   5000
///                     ]
///                 }
///
/// Example session
///
/// In this example session, 3 clients connect to the server.
/// Clients 1 & 2 are cameras on road 123, with a 60 mph speed limit.
/// Client 3 is a ticket dispatcher for road 123. The car with number plate UN1X was observed
///  passing the first camera at timestamp 0, and passing the second camera 45 seconds later.
/// It travelled 1 mile in 45 seconds, which means it was travelling at 80 mph.
/// This is in excess of the speed limit, so a ticket is dispatched.
///
/// "-->" denotes messages from the server to the client, and "<--" denotes messages from the client to the server.
/// Client 1: camera at mile 8
///
/// Hexadecimal:
/// <-- 80 00 7b 00 08 00 3c
/// <-- 20 04 55 4e 31 58 00 00 00 00
///
/// Decoded:
/// <-- IAmCamera{road: 123, mile: 8, limit: 60}
/// <-- Plate{plate: "UN1X", timestamp: 0}
///
/// Client 2: camera at mile 9
///
/// Hexadecimal:
/// <-- 80 00 7b 00 09 00 3c
/// <-- 20 04 55 4e 31 58 00 00 00 2d
///
/// Decoded:
/// <-- IAmCamera{road: 123, mile: 9, limit: 60}
/// <-- Plate{plate: "UN1X", timestamp: 45}
///
/// Client 3: ticket dispatcher
///
/// Hexadecimal:
/// <-- 81 01 00 7b
/// --> 21 04 55 4e 31 58 00 7b 00 08 00 00 00 00 00 09 00 00 00 2d 1f 40
///
/// Decoded:
/// <-- IAmDispatcher{roads: [123]}
/// --> Ticket{plate: "UN1X", road: 123, mile1: 8, timestamp1: 0, mile2: 9, timestamp2: 45, speed: 8000}
///
/// Details
///
/// Dispatchers
///
/// When the server generates a ticket for a road that has multiple connected dispatchers,
///  the server may choose between them arbitrarily, but must not ever send the same ticket twice.
///
/// If the server sends a ticket but the dispatcher disconnects before it receives it,
///  then the ticket simply gets lost and the driver escapes punishment.
///
/// If the server generates a ticket for a road that has no connected dispatcher,
///  it must store the ticket and deliver it once a dispatcher for that road is available.
///
/// Unreliable cameras
///
/// Sometimes number plates aren't spotted (maybe they were obscured, or the image was blurry),
///  so a car can skip one or more cameras and reappear later on.
/// You must still generate a ticket if its average speed exceeded the limit between any
///  pair of observations on the same road, even if the observations were not from adjacent cameras.
///
///  No shortcuts
///
/// The fastest legal route between any pair of cameras that are on the same road is to use the
///  road that those cameras are on; you don't need to worry about falsely ticketing drivers
///  who may have left a road and rejoined it.
///
/// Only 1 ticket per car per day
///
/// The server may send no more than 1 ticket for any given car on any given day.
///
/// Where a ticket spans multiple days, the ticket is considered to apply to every day
///  from the start to the end day, including the end day.
/// This means that where there is a choice of observations to include in a ticket,
///  it is sometimes possible for the server to choose either to send a ticket for each day,
///  or to send a single ticket that spans both days: either behaviour is acceptable.
/// (But to maximise revenues, you may prefer to send as many tickets as possible).
///
/// Since timestamps do not count leap seconds, days are defined by floor(timestamp / 86400).
///
/// Rounding
///
/// It is always required to ticket a car that is exceeding the speed limit by 0.5 mph or more
///
/// In cases where the car is exceeding the speed limit by less than 0.5 mph, it is acceptable to omit the ticket.
///
/// It is never acceptable to ticket a car that had an average speed below the speed limit.
///
///  Overflow
///
/// In principle, a car travelling in excess of 655.35 mph would cause the server to generate a ticket
///  with an incorrect speed.
/// Fortunately nobody on Freedom Island has a fast enough car, so you don't need to worry about it.
#[derive(Debug, Default)]
pub struct SpeedDaemonServer {
    messages: Arc<Mutex<VecDeque<MessageType>>>,
    clients: Arc<Mutex<HashMap<SocketAddr, ClientType>>>,
    cameras: Arc<Mutex<HashMap<u16, ClientInfo>>>,
    dispatchers: Arc<Mutex<HashMap<u16, ClientInfo>>>,
}

#[async_trait]
impl Server for SpeedDaemonServer {
    /// Run the server
    async fn run(&mut self, addr: &str) -> Result<(), ServerErrorKind> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|_| ServerErrorKind::BindFail)?;

        println!("Listening on {:?}", addr);

        let messages = self.messages.clone();
        let clients = self.clients.clone();
        let cameras = self.cameras.clone();
        let dispatchers = self.dispatchers.clone();

        let (tx, rx) = mpsc::unbounded_channel::<InternalMessage>();

        let tx = Arc::new(Mutex::new(tx));

        // A new task is spawned for processing
        tokio::spawn(async move {
            println!("Processing thread started ...");
            let mut consumer = ProxyConsumer::new(clients, cameras, dispatchers);

            consumer.run(rx, messages).await
        });

        loop {
            println!("Waiting for connection ...");

            // The second item contains the IP and port of the new connection.
            let (stream, addr) = listener.accept().await.unwrap();

            println!("Connection open\n");

            let messages = self.messages.clone();
            let clients = self.clients.clone();
            let cameras = self.cameras.clone();
            let dispatchers = self.dispatchers.clone();
            let consumer = tx.clone();

            // A new task is spawned for each inbound socket. The socket is
            // moved to the new task and processed there.
            tokio::spawn(async move {
                let mut client = Client::new(clients, cameras, dispatchers);

                client.run(consumer, messages, addr, stream).await
            });
        }
    }
}

#[derive(Debug)]
enum InternalMessage {
    NewClient,
    ErrorMessage,

}

#[derive(Debug)]
struct ProxyConsumer {
    clients: Arc<Mutex<HashMap<SocketAddr, ClientType>>>,
    cameras: Arc<Mutex<HashMap<u16, ClientInfo>>>,
    dispatchers: Arc<Mutex<HashMap<u16, ClientInfo>>>,
}

impl ProxyConsumer {
    fn new(
        clients: Arc<Mutex<HashMap<SocketAddr, ClientType>>>,
        cameras: Arc<Mutex<HashMap<u16, ClientInfo>>>,
        dispatchers: Arc<Mutex<HashMap<u16, ClientInfo>>>,
    ) -> Self {
        Self {
            clients,
            cameras,
            dispatchers,
        }
    }

    async fn run(&mut self, rx: mpsc::UnboundedReceiver<InternalMessage>, messages: Arc<Mutex<VecDeque<MessageType>>>) -> Result<(), ServerErrorKind> {
        let mut rx = rx;

        while let Some(msg) = rx.recv().await {
            match msg {
                InternalMessage::NewClient => {
                    dbg!("New client");
                }
                InternalMessage::ErrorMessage => {
                    dbg!("Error message");
                }
            }
        }

        dbg!("Processing thread finished ...");

        Ok(())
    }
}

#[derive(Debug, Default)]
enum ClientType {
    #[default]
    Unkwown,
    Camera,
    Dispatcher,
}

#[derive(Debug)]
struct ClientInfo {
    client_type: ClientType,
    tx: mpsc::UnboundedSender<Vec<u8>>,
}

impl ClientInfo {
    fn new(client_type: ClientType, tx: mpsc::UnboundedSender<Vec<u8>>) -> Self {
        Self { client_type, tx }
    }
}

#[derive(Debug)]
struct Client {
    clients: Arc<Mutex<HashMap<SocketAddr, ClientType>>>,
    cameras: Arc<Mutex<HashMap<u16, ClientInfo>>>,
    dispatchers: Arc<Mutex<HashMap<u16, ClientInfo>>>,
}

impl Client {
    fn new(
        clients: Arc<Mutex<HashMap<SocketAddr, ClientType>>>,
        cameras: Arc<Mutex<HashMap<u16, ClientInfo>>>,
        dispatchers: Arc<Mutex<HashMap<u16, ClientInfo>>>,
    ) -> Self {
        Self {
            clients,
            cameras,
            dispatchers,
        }
    }

    async fn run(
        &mut self,
        consumer: Arc<Mutex<UnboundedSender<InternalMessage>>>,
        messages: Arc<Mutex<VecDeque<MessageType>>>,
        addr: SocketAddr,
        stream: TcpStream) -> Result<(), ServerErrorKind> {

        let mut client_type = ClientType::Unkwown;
        let mut stream = BufStream::new(stream);
        let mut line = vec![];
        let mut should_continue = true;

        let (tx, mut rx) = mpsc::unbounded_channel::<InternalMessage>();

        // Store the client info
        self.clients.lock().await.insert(addr, client_type);

        // Send a message to the consumer
        consumer.lock().await.send(InternalMessage::NewClient).unwrap();

        while should_continue {
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
                        stream
                            .write_all(&response)
                            .await
                            .map_err(|_| ServerErrorKind::WriteFail)?;

                        // Flush the buffer to ensure it is sent
                        stream
                            .flush()
                            .await
                            .map_err(|_| ServerErrorKind::WriteFail)?;
                    }
                }
                // If there's a request incoming
                result = stream.read_until(b'\n', &mut line) => {
                    let read_len = result.map_err(|_| ServerErrorKind::ReadFail)?;

                    if read_len > 0 {
                        // Process the received request/line
                        if let Some(msg) = MessageType::from_bytes(&line) {
                            messages.lock().await.push_back(msg);
                        }
                    } else {
                        should_continue = false;
                    }

                    line.clear();
                }
            }
        }

        dbg!("Connection closed: {}", addr);

        Ok(())
    }
}


#[derive(Debug)]
enum MessageType {
    Error(String),
}

impl MessageType {
    fn from_bytes(msg: &[u8]) -> Option<Self> {
        let mut msg_iter = msg.iter();

        match msg_iter.next() {
            // Error
            Some(0x10_u8) => {
                let str_len = msg_iter.next().unwrap();
                let s = String::from_utf8(msg_iter.take(*str_len as usize).cloned().collect()).unwrap();

                Some(MessageType::Error(s))
            }

            // Unknown
            Some(_) | None => {
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_message_type_from_bytes_error_success() {
        let msg = vec![0x10, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f];
        let msg_type = MessageType::from_bytes(&msg);

        assert!(matches!(msg_type, Some(MessageType::Error(_))));
    }

    #[test]
    fn test_message_type_from_bytes_unknown_type_fail() {
        let msg = vec![0x11, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f];
        let msg_type = MessageType::from_bytes(&msg);

        assert!(matches!(msg_type, None));
    }

    #[test]
    fn test_message_type_from_bytes_error_invalid_length_fail() {
        let msg = vec![0x10, 0x06, 0x48, 0x65, 0x6c, 0x6c, 0x6f];
        let msg_type = MessageType::from_bytes(&msg);

        assert!(matches!(msg_type, None));
    }
}
