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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionResult {
    pub output_dir: PathBuf,
    pub link_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    pub candidates: Vec<Candidate>,
    pub collection: Option<CollectionResult>,
}

impl RunResult {
    pub(crate) fn new(candidates: Vec<Candidate>, collection: Option<CollectionResult>) -> Self {
        Self {
            candidates,
            collection,
        }
    }
}
