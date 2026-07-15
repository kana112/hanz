mod hash;
mod name;

use anyhow::Result;

use crate::candidate::Candidate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DetectionOptions {
    pub(crate) by_name: bool,
    pub(crate) by_hash: bool,
}

pub(crate) fn detect_candidates(
    files: &[std::path::PathBuf],
    options: DetectionOptions,
) -> Result<Vec<Candidate>> {
    let mut candidates = Vec::new();
    if options.by_name {
        candidates.extend(name::detect_by_name(files));
    }
    if options.by_hash {
        candidates.extend(hash::detect_by_hash(files)?);
    }
    Ok(candidates)
}
