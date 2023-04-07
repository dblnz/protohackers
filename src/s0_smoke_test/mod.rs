#[cfg(feature = "s0")]
pub use self::smoke_test::*;

#[cfg(feature = "s0")]
mod smoke_test {
    use crate::solution::{ProtoHSolution, SolutionError};

    /// Smoke Test - TCP Echo Service
    ///
    /// This Protocol implies replying to the requests with
    /// the same data received
    #[derive(Debug)]
    pub struct SmokeTestSolution;

    impl ProtoHSolution for SmokeTestSolution {
        fn process_request(&mut self, line: &[u8]) -> Result<Vec<u8>, SolutionError> {
            Ok(line.to_vec())
        }
    }
}
