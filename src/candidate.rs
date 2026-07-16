use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub path: PathBuf,
    pub kind: CandidateKind,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    Name,
    Hash,
    DirectoryHash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    pub candidates: Vec<Candidate>,
}

impl RunResult {
    pub(crate) fn new(candidates: Vec<Candidate>) -> Self {
        Self { candidates }
    }
}
