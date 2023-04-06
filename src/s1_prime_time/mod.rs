use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::solution::{ProtoHSolution, SolutionError};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ConformingReqObj {
    method: String,
    number: i64,
}
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ConformingRespObj {
    method: String,
    prime: bool,
}

#[derive(Debug, PartialEq)]
enum Request {
    ConformingReq { method: String, number: i64 },
    MalformedReq,
}

#[derive(Debug, PartialEq)]
enum Response {
    ConformingResp { method: String, prime: bool },
    MalformedResp,
}

impl Request {
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

    fn process(self) -> Response {
        match self {
            Request::ConformingReq { method, number } => Response::ConformingResp {
                method,
                prime: Request::is_prime(number),
            },
            Request::MalformedReq => Response::MalformedResp,
        }
    }

    fn is_prime(number: i64) -> bool {
        if number < 2 {
            false
        } else {
            !(2..number / 2).any(|n| number % n == 0)
        }
    }
}

impl Response {
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

#[derive(Debug)]
pub struct PrimeTimeSolution;

#[async_trait]
impl ProtoHSolution for PrimeTimeSolution {
    fn process_request(&mut self, line: &[u8]) -> Result<Vec<u8>, SolutionError> {
        // Construct a request from the u8 vec
        let req = Request::from_bytes(line);

        // Consume the request and construct a response
        let resp = req.process();

        // Get a string from the response
        match resp {
            Response::ConformingResp {
                method: _,
                prime: _,
            } => Ok(resp.into_bytes()),
            Response::MalformedResp => Err(SolutionError::InvalidRequest(resp.into_bytes())),
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
        assert_eq!(obj.number, 123);
    }

    #[test]
    fn test_deserialize_inverted_req() {
        let line = b"{\"number\":2,\"method\":\"isPrime\"}";

        let obj: ConformingReqObj = serde_json::from_slice(line).unwrap();
        assert_eq!(obj.method, "isPrime");
        assert_eq!(obj.number, 2);
    }

    #[test]
    fn test_deserialize_inverted_newline_req() {
        let line = b"{\"number\":2,\"method\":\"isPrime\"}\n";

        let obj: ConformingReqObj = serde_json::from_slice(line).unwrap();
        assert_eq!(obj.method, "isPrime");
        assert_eq!(obj.number, 2);
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
                number: 123
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
                number: 2
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
                number: 11
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
                number: 2
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
                number: 9
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
                number: 2
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
                number: 778013
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
                number: 2
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
                number: 0
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
                number: 1
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
                number: -3
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
