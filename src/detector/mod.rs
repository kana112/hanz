mod hash;
mod name;

use anyhow::Result;

use crate::candidate::{Candidate, CandidateKind};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DetectionOptions {
    pub(crate) by_name: bool,
    pub(crate) by_hash: bool,
}

pub(crate) fn detect_candidates(
    files: &[PathBuf],
    directories: &[PathBuf],
    options: DetectionOptions,
) -> Result<Vec<Candidate>> {
    let mut candidates = Vec::new();
    if options.by_name {
        candidates.extend(name::detect_by_name(files));
    }
    if options.by_hash {
        let hash_candidates = hash::detect_by_hash(files, directories)?;
        let duplicate_directories = hash_candidates
            .iter()
            .filter(|candidate| candidate.kind == CandidateKind::DirectoryHash)
            .map(|candidate| candidate.path.as_path())
            .collect::<Vec<_>>();
        candidates.retain(|candidate| {
            !duplicate_directories
                .iter()
                .any(|directory| candidate.path.starts_with(directory))
        });
        candidates.extend(hash_candidates);
    }
    Ok(candidates)
}
