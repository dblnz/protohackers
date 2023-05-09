use async_trait::async_trait;
use std::collections::HashMap;

use server::{Server, ServerErrorKind};
use tokio::net::UdpSocket;

/// Unusual Database Program
///
/// It's your first day on the job. Your predecessor, Ken,
/// left in mysterious circumstances, but not before coming up
/// with a protocol for the new key-value database.
/// You have some doubts about Ken's motivations,
/// but there's no time for questions! Let's implement his protocol.
///
/// Ken's strange database is a key-value store accessed over UDP.
/// Since UDP does not provide retransmission of dropped packets,
/// and neither does Ken's protocol, clients have to be careful not
/// to send requests too fast, and have to accept that some requests
/// or responses may be dropped.
///
/// Each request, and each response, is a single UDP packet.
///
/// There are only two types of request: insert and retrieve.
/// Insert allows a client to insert a value for a key, and retrieve
/// allows a client to retrieve the value for a key.
///
/// Insert
///
/// A request is an insert if it contains an equals sign ("=", or ASCII 61).
///
/// The first equals sign separates the key from the value.
/// This means keys can not contain the equals sign. Other than the equals sign,
/// keys can be made up of any arbitrary characters. The empty string is a valid key.
///
/// Subsequent equals signs (if any) are included in the value.
/// The value can be any arbitrary data, including the empty string.
///
/// For example:
///
/// foo=bar will insert a key foo with value "bar".
/// foo=bar=baz will insert a key foo with value "bar=baz".
/// foo= will insert a key foo with value "" (i.e. the empty string).
/// foo=== will insert a key foo with value "==".
/// =foo will insert a key of the empty string with value "foo".
///
/// If the server receives an insert request for a key that already exists,
/// the stored value must be updated to the new value.
///
/// An insert request does not yield a response.
///
/// Retrieve
///
/// A request that does not contain an equals sign is a retrieve request.
///
/// In response to a retrieve request, the server must send back the key
/// and its corresponding value, separated by an equals sign.
/// Responses must be sent to the IP address and port number that the request
/// originated from, and must be sent from the IP address and port number that the request was sent to.
///
/// If a requests is for a key that has been inserted multiple times,
/// the server must return the most recent value.
///
/// If a request attempts to retrieve a key for which no value exists,
/// the server can either return a response as if the key had the empty value (e.g. "key="),
/// or return no response at all.
///
/// Example request:
///
/// `message`
///
/// Example response:
///
/// `message=Hello,world!`
///
/// Version reporting
///
/// The server must implement the special key "version". This should identify the
/// server software in some way (for example, it could contain the software name
/// and version number). It must not be the empty string.
///
/// Attempts by clients to modify the version must be ignored.
///
/// Example request:
///
/// `version`
///
/// Example response:
///
/// `version=Ken's Key-Value Store 1.0`
///
/// Other requirements
///
/// All requests and responses must be shorter than 1000 bytes.
///
/// Issues related to UDP packets being dropped, delayed, or reordered are
/// considered to be the client's problem.
/// The server should act as if it assumes that UDP works reliably.
#[derive(Debug)]
pub struct UnusualDatabaseProgramServer {
    db: HashMap<String, String>,
}

#[async_trait]
impl Server for UnusualDatabaseProgramServer {
    /// Method that starts the server
    async fn run(&mut self, addr: &str) -> Result<(), ServerErrorKind> {
        let listener = UdpSocket::bind(addr)
            .await
            .map_err(|_| ServerErrorKind::BindFail)?;

        println!("Listening on {:?}", addr);

        let mut buf = [0; 1024];
        loop {
            let (len, addr) = listener
                .recv_from(&mut buf)
                .await
                .map_err(|_| ServerErrorKind::ReadFail)?;
            println!("{:?} bytes received from {:?}", len, addr);

            let resp = self.process_request(&buf[..len]);

            if !resp.is_empty() {
                let len = listener
                    .send_to(&resp, addr)
                    .await
                    .map_err(|_| ServerErrorKind::WriteFail)?;
                println!("{:?} bytes sent", len);
            }
        }
    }
}

impl Default for UnusualDatabaseProgramServer {
    fn default() -> Self {
        let mut db = HashMap::new();
        db.insert("version".to_string(), "Ken's Key-Value Store 1.0".to_string());

        Self {
            db,
        }
    }
}

impl UnusualDatabaseProgramServer {
    fn process_request(&mut self, msg: &[u8]) -> Vec<u8> {
        let mut msg = String::from_utf8(msg.to_vec()).unwrap();

        // Remove the trailing newline
        msg = msg
            .strip_suffix("\r\n")
            .or(msg.strip_suffix('\n'))
            .unwrap_or(&msg)
            .to_string();

        let parts = msg.split('=').collect::<Vec<_>>();

        // insert request
        if parts.len() > 1 {
            if parts[0] != "version" {
                self.db.insert(parts[0].to_string(), parts[1..].join("=").to_string());
            }

            vec![]
        }
        // retrieve request
        else {
            let key = parts[0];

            let default = "".to_string();
            let value = self.db.get(key).unwrap_or(&default);

            format!("{}={}", key, value).into_bytes()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_process_request_insert_success() {
        let mut server = UnusualDatabaseProgramServer::default();

        let msg = "foo=bar";
        let resp = server.process_request(msg.as_bytes());

        assert_eq!(resp, vec![]);
        assert_eq!(server.db.get("foo"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_process_request_insert_empty_key_success() {
        let mut server = UnusualDatabaseProgramServer::default();

        let msg = "=bar";
        let resp = server.process_request(msg.as_bytes());

        assert_eq!(resp, vec![]);
        assert_eq!(server.db.get(""), Some(&"bar".to_string()));
    }

    #[test]
    fn test_process_request_insert_multiple_equals_success() {
        let mut server = UnusualDatabaseProgramServer::default();

        let msg = "foo=bar=baz";
        let resp = server.process_request(msg.as_bytes());

        assert_eq!(resp, vec![]);
        assert_eq!(server.db.get("foo"), Some(&"bar=baz".to_string()));
    }

    #[test]
    fn test_process_request_insert_multiple_consecutive_equals_success() {
        let mut server = UnusualDatabaseProgramServer::default();

        let msg = "foo===";
        let resp = server.process_request(msg.as_bytes());

        assert_eq!(resp, vec![]);
        assert_eq!(server.db.get("foo"), Some(&"==".to_string()));
    }

    #[test]
    fn test_process_request_insert_empty_value_success() {
        let mut server = UnusualDatabaseProgramServer::default();

        let msg = "foo=";
        let resp = server.process_request(msg.as_bytes());

        assert_eq!(resp, vec![]);
        assert_eq!(server.db.get("foo"), Some(&"".to_string()));
    }

    #[test]
    fn test_process_request_retrieve_key_value_success() {
        let mut server = UnusualDatabaseProgramServer::default();

        let msg = "foo=bar";
        let resp = server.process_request(msg.as_bytes());

        assert_eq!(resp, vec![]);
        assert_eq!(server.db.get("foo"), Some(&"bar".to_string()));

        let resp = server.process_request("foo".as_bytes());
        assert_eq!(resp, "foo=bar".as_bytes());
    }

    #[test]
    fn test_process_request_retrieve_version_success() {
        let mut server = UnusualDatabaseProgramServer::default();

        let resp = server.process_request("version".as_bytes());
        assert_eq!(resp, "version=Ken's Key-Value Store 1.0".as_bytes());
    }

    #[test]
    fn test_process_request_insert_version_success() {
        let mut server = UnusualDatabaseProgramServer::default();

        let msg = "version=foo";
        let resp = server.process_request(msg.as_bytes());

        assert_eq!(resp, vec![]);
        assert_eq!(server.db.get("version"), Some(&"Ken's Key-Value Store 1.0".to_string()));
    }
}