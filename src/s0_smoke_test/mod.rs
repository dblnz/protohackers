use crate::solution::ProtoHSolution;

#[derive(Debug)]
pub struct SmokeTestSolution;

impl ProtoHSolution for SmokeTestSolution {
    fn process_request(&mut self, line: &[u8]) -> Vec<u8> {
        line.to_vec()
    }
}
