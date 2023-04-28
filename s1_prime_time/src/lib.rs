use serde::{Deserialize, Serialize};
use server::{Action, Protocol, SolutionError, RequestDelimiter};

/// Conforming Request object
/// Used for deserializing JSON bytes received
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ConformingReqObj {
    pub method: String,
    pub number: f64,
}

/// Conforming Response object
/// Used for serializing JSON before sending
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ConformingRespObj {
    method: String,
    prime: bool,
}

/// Request type
/// Constructed based on bytes and verified if it
/// satisfies the solution conditions
#[derive(Debug, PartialEq)]
enum Request {
    ConformingReq { method: String, number: f64 },
    MalformedReq,
}

/// Response type
/// Constructed based on a processed request
#[derive(Debug, PartialEq)]
enum Response {
    ConformingResp { method: String, prime: bool },
    MalformedResp,
}

impl Request {
    /// Creates a `Request` from provided bytes
    /// and verifies if it satisfies the solution conditions
    ///
    /// If it does, it returns a `Request::ConformingReq`
    /// Otherwise, it returns a `Request::MalformedReq`
    fn from_bytes(line: &[u8]) -> Request {
        let obj = serde_json::from_slice::<ConformingReqObj>(line).ok();

        match obj {
            Some(ConformingReqObj { method, number }) => {
                if method == "isPrime" {
                    Request::ConformingReq { method, number }
                } else {
                    Request::MalformedReq
                }
            }
            None => Request::MalformedReq,
        }
    }

    /// Processes the request and returns a `Response`
    fn process(self) -> Response {
        match self {
            Request::ConformingReq { method, number } => Response::ConformingResp {
                method,
                prime: is_prime(number),
            },
            Request::MalformedReq => Response::MalformedResp,
        }
    }
}

impl Response {
    /// Converts the response into bytes ready to be sent
    fn into_bytes(self) -> Vec<u8> {
        match self {
            Response::ConformingResp { method, prime } => {
                let obj = ConformingRespObj { method, prime };
                let mut res = serde_json::to_string(&obj).unwrap();

                // Add newline
                res.push('\n');
                res.as_bytes().to_vec()
            }
            Response::MalformedResp => b"malformed\n".to_vec(),
        }
    }
}

/// Checks if a number is prime
fn is_prime(number: f64) -> bool {
    let n = number;
    let number = number as i64;

    if n.fract() != 0.0 || number < 2 {
        false
    } else {
        let end = f64::sqrt(number as f64) as i64;

        !(2..=end).any(|n| number % n == 0)
    }
}

/// Prime Time
///
/// This Service receives JSON Formatted objects that specify a number to
/// be checked if is prime
///
/// A conforming request object has the required field method,
/// which must always contain the string `isPrime`, and the required field number,
/// which must contain a number.
/// Any JSON number is a valid number, including floating-point values.
///
/// Input Example:
/// `{"method":"isPrime","number":123}`
///
/// A request is malformed if it is not a well-formed JSON object,
/// if any required field is missing, if the method name is not `isPrime`,
/// or if the number value is not a number.
///
/// Extraneous fields are to be ignored.
///
/// A conforming response object has the required field method,
/// which must always contain the string `isPrime`, and the required field prime,
/// which must contain a boolean value: true if the number in the request was prime,
/// false if it was not.
///
/// Output Example:
///  `{"method":"isPrime","prime":false}`
///
/// A response is malformed if it is not a well-formed JSON object, if any required field is missing,
/// if the method name is not `isPrime`, or if the prime value is not a boolean.
///
/// Accept TCP connections.
///
/// Whenever you receive a conforming request, send back a correct response, and wait for another request.
///
/// Whenever you receive a malformed request, send back a single malformed response, and disconnect the client.
///
/// Make sure you can handle at least 5 simultaneous clients.
#[derive(Debug, Default)]
pub struct PrimeTimeSolution;

/// Implement the solution trait for the solution
impl PrimeTimeSolution {
    /// Create a new instance of the solution
    pub fn new() -> Self {
        Self::default()
    }
}

/// Implement the protocol trait for the solution
/// This is where the custom logic for the solution is implemented
impl Protocol for PrimeTimeSolution {
    /// The delimiter is a newline character
    fn get_delimiter(&self) -> RequestDelimiter {
        RequestDelimiter::UntilChar(b'\n')
    }

    /// Process a request and return a response
    fn process_request(&mut self, line: &[u8]) -> Result<Action, SolutionError> {
        // Construct a request from the u8 vec
        let req = Request::from_bytes(line);

        // Consume the request and construct a response
        let resp = req.process();

        // return an error if the response is malformed
        // otherwise return the response
        match resp {
            Response::ConformingResp { .. } => Ok(Action::Reply(resp.into_bytes())),
            Response::MalformedResp => Err(SolutionError::MalformedRequest(resp.into_bytes())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_deserialize_req() {
        let line = b"{\"method\":\"isPrime\",\"number\":123}";

        let obj: ConformingReqObj = serde_json::from_slice(line).unwrap();
        assert_eq!(obj.method, "isPrime");
        assert_eq!(obj.number, 123f64);
    }

    #[test]
    fn test_deserialize_inverted_req() {
        let line = b"{\"number\":2,\"method\":\"isPrime\"}";

        let obj: ConformingReqObj = serde_json::from_slice(line).unwrap();
        assert_eq!(obj.method, "isPrime");
        assert_eq!(obj.number, 2f64);
    }

    #[test]
    fn test_deserialize_inverted_newline_req() {
        let line = b"{\"number\":2,\"method\":\"isPrime\"}\n";

        let obj: ConformingReqObj = serde_json::from_slice(line).unwrap();
        assert_eq!(obj.method, "isPrime");
        assert_eq!(obj.number, 2f64);
    }

    #[test]
    fn test_serialize_deserialize_resp() {
        let obj_0 = ConformingRespObj {
            method: "isPrime".to_string(),
            prime: true,
        };

        let ser = serde_json::to_string(&obj_0).unwrap().as_bytes().to_vec();
        let obj_1: ConformingRespObj = serde_json::from_slice(&ser).unwrap();

        assert_eq!(obj_0, obj_1);
    }

    #[test]
    fn test_req_valid_success() {
        let line = b"{\"method\":\"isPrime\",\"number\":123}";
        let req = Request::from_bytes(line);

        assert_eq!(
            req,
            Request::ConformingReq {
                method: "isPrime".to_string(),
                number: 123f64
            }
        );
    }

    #[test]
    fn test_req_valid_inverted_success() {
        let line = b"{\"number\":2,\"method\":\"isPrime\"}";
        let req = Request::from_bytes(line);

        assert_eq!(
            req,
            Request::ConformingReq {
                method: "isPrime".to_string(),
                number: 2f64
            }
        );
    }

    #[test]
    fn test_req_malformed_json_error() {
        let line = b"\"method\":\"isPrim\",\"number\":123}";
        let req = Request::from_bytes(line);

        assert_eq!(req, Request::MalformedReq);
    }

    #[test]
    fn test_req_malformed_invalid_method_error() {
        let line = b"{\"method\":\"isPrim\",\"number\":123}";
        let req = Request::from_bytes(line);

        assert_eq!(req, Request::MalformedReq);
    }

    #[test]
    fn test_req_malformed_number_error() {
        let line = b"{\"method\":\"isPrime\",\"number\":\"123\"}";
        let req = Request::from_bytes(line);

        assert_eq!(req, Request::MalformedReq);
    }

    #[test]
    fn test_req_process_prime_success() {
        let line = b"{\"method\":\"isPrime\",\"number\":11}";
        let req = Request::from_bytes(line);

        assert_eq!(
            req,
            Request::ConformingReq {
                method: "isPrime".to_string(),
                number: 11f64
            }
        );
        assert_eq!(
            req.process(),
            Response::ConformingResp {
                method: "isPrime".to_string(),
                prime: true
            }
        );
    }

    #[test]
    fn test_req_process_prime_inverted_success() {
        let line = b"{\"number\":2,\"method\":\"isPrime\"}";
        let req = Request::from_bytes(line);

        assert_eq!(
            req,
            Request::ConformingReq {
                method: "isPrime".to_string(),
                number: 2f64
            }
        );
        assert_eq!(
            req.process(),
            Response::ConformingResp {
                method: "isPrime".to_string(),
                prime: true
            }
        );
    }

    #[test]
    fn test_req_process_not_prime_success() {
        let line = b"{\"method\":\"isPrime\",\"number\":9}";
        let req = Request::from_bytes(line);

        assert_eq!(
            req,
            Request::ConformingReq {
                method: "isPrime".to_string(),
                number: 9f64
            }
        );
        assert_eq!(
            req.process(),
            Response::ConformingResp {
                method: "isPrime".to_string(),
                prime: false
            }
        );
    }

    #[test]
    fn test_req_process_prime_2_success() {
        let line = b"{\"method\":\"isPrime\",\"number\":2}";
        let req = Request::from_bytes(line);

        assert_eq!(
            req,
            Request::ConformingReq {
                method: "isPrime".to_string(),
                number: 2f64
            }
        );
        assert_eq!(
            req.process(),
            Response::ConformingResp {
                method: "isPrime".to_string(),
                prime: true
            }
        );
    }

    #[test]
    fn test_req_to_vec_prime_success() {
        let line = b"{\"method\":\"isPrime\",\"number\":778013}\n";
        let req = Request::from_bytes(line);

        assert_eq!(
            req,
            Request::ConformingReq {
                method: "isPrime".to_string(),
                number: 778013f64
            }
        );
        let resp = req.process();
        assert_eq!(
            resp,
            Response::ConformingResp {
                method: "isPrime".to_string(),
                prime: true
            }
        );
        assert_eq!(
            resp.into_bytes(),
            "{\"method\":\"isPrime\",\"prime\":true}\n"
                .to_string()
                .as_bytes()
                .to_vec()
        );
    }

    #[test]
    fn test_req_to_vec_prime_inverted_success() {
        let line = b"{\"number\":2,\"method\":\"isPrime\"}";
        let req = Request::from_bytes(line);

        assert_eq!(
            req,
            Request::ConformingReq {
                method: "isPrime".to_string(),
                number: 2f64
            }
        );
        let resp = req.process();
        assert_eq!(
            resp,
            Response::ConformingResp {
                method: "isPrime".to_string(),
                prime: true
            }
        );
        assert_eq!(
            resp.into_bytes(),
            "{\"method\":\"isPrime\",\"prime\":true}\n"
                .to_string()
                .as_bytes()
                .to_vec()
        );
    }

    #[test]
    fn test_req_to_vec_prime_0_success() {
        let line = b"{\"number\":0,\"method\":\"isPrime\"}";
        let req = Request::from_bytes(line);

        assert_eq!(
            req,
            Request::ConformingReq {
                method: "isPrime".to_string(),
                number: 0f64
            }
        );
        let resp = req.process();
        assert_eq!(
            resp,
            Response::ConformingResp {
                method: "isPrime".to_string(),
                prime: false
            }
        );
        assert_eq!(
            resp.into_bytes(),
            "{\"method\":\"isPrime\",\"prime\":false}\n"
                .to_string()
                .as_bytes()
                .to_vec()
        );
    }

    #[test]
    fn test_req_to_vec_prime_1_success() {
        let line = b"{\"number\":1,\"method\":\"isPrime\"}";
        let req = Request::from_bytes(line);

        assert_eq!(
            req,
            Request::ConformingReq {
                method: "isPrime".to_string(),
                number: 1f64
            }
        );
        let resp = req.process();
        assert_eq!(
            resp,
            Response::ConformingResp {
                method: "isPrime".to_string(),
                prime: false
            }
        );
        assert_eq!(
            resp.into_bytes(),
            "{\"method\":\"isPrime\",\"prime\":false}\n"
                .to_string()
                .as_bytes()
                .to_vec()
        );
    }

    #[test]
    fn test_req_to_vec_not_prime_negative_success() {
        let line = b"{\"number\":-3,\"method\":\"isPrime\"}";
        let req = Request::from_bytes(line);

        assert_eq!(
            req,
            Request::ConformingReq {
                method: "isPrime".to_string(),
                number: -3f64
            }
        );
        let resp = req.process();
        assert_eq!(
            resp,
            Response::ConformingResp {
                method: "isPrime".to_string(),
                prime: false
            }
        );
        assert_eq!(
            resp.into_bytes(),
            "{\"method\":\"isPrime\",\"prime\":false}\n"
                .to_string()
                .as_bytes()
                .to_vec()
        );
    }

    #[test]
    fn test_req_to_vec_float_success() {
        let line = b"{\"method\":\"isPrime\",\"number\":4224223.1234}\n";
        let req = Request::from_bytes(line);

        assert_eq!(
            req,
            Request::ConformingReq {
                method: "isPrime".to_string(),
                number: 4224223.1234f64
            }
        );
        let resp = req.process();
        assert_eq!(
            resp,
            Response::ConformingResp {
                method: "isPrime".to_string(),
                prime: false
            }
        );
        assert_eq!(
            resp.into_bytes(),
            "{\"method\":\"isPrime\",\"prime\":false}\n"
                .to_string()
                .as_bytes()
                .to_vec()
        );
    }
}

