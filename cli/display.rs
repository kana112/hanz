use std::io::{self, Write};

use anyhow::Result;
use hanz::{Candidate, CandidateKind, RunResult};

const NAME_LABEL: &str = "NAME";
const HASH_LABEL: &str = "HASH";
const DIRECTORY_HASH_LABEL: &str = "DIR_HASH";

pub(super) fn write_result(result: &RunResult) -> Result<()> {
    let output = format_run_output(result);
    io::stdout().lock().write_all(output.as_bytes())?;
    Ok(())
}

fn format_candidates(candidates: &[Candidate]) -> String {
    if candidates.is_empty() {
        return "No candidates found.\n".to_owned();
    }
    candidates.iter().map(format_candidate).collect()
}

fn format_candidate(candidate: &Candidate) -> String {
    let mut output = format!(
        "{}  {}\n",
        candidate_label(candidate.kind),
        candidate.path.display()
    );
    for line in candidate.reason.lines() {
        output.push_str("      ");
        output.push_str(line);
        output.push('\n');
    }
    output.push('\n');
    output
}

fn format_run_output(result: &RunResult) -> String {
    let mut output = format_candidates(&result.candidates);
    if let Some(collection) = &result.collection {
        append_collection_summary(&mut output, collection);
    }
    output
}

fn append_collection_summary(output: &mut String, collection: &hanz::CollectionResult) {
    output.push_str(&format!(
        "COLLECT  {} candidate link(s) in {}\n",
        collection.link_count,
        collection.output_dir.display()
    ));
}

fn candidate_label(kind: CandidateKind) -> &'static str {
    match kind {
        CandidateKind::Name => NAME_LABEL,
        CandidateKind::Hash => HASH_LABEL,
        CandidateKind::DirectoryHash => DIRECTORY_HASH_LABEL,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use hanz::CollectionResult;

    fn candidate(kind: CandidateKind, path: &str, reason: &str) -> Candidate {
        Candidate {
            path: PathBuf::from(path),
            kind,
            reason: reason.to_owned(),
        }
    }

    #[test]
    fn empty_candidate_output_is_explicit() {
        let result = RunResult {
            candidates: Vec::new(),
            collection: None,
        };
        assert_eq!(format_run_output(&result), "No candidates found.\n");
    }

    #[test]
    fn candidate_output_contains_kind_path_and_reason() {
        let result = RunResult {
            candidates: vec![candidate(
                CandidateKind::Name,
                "report (1).pdf",
                "reason: duplicate-like filename",
            )],
            collection: None,
        };
        let output = format_run_output(&result);
        assert!(output.contains("NAME  report (1).pdf"));
        assert!(output.contains("      reason: duplicate-like filename"));
    }

    #[test]
    fn collection_output_contains_summary() {
        let result = RunResult {
            candidates: vec![candidate(CandidateKind::Name, "source.txt", "reason")],
            collection: Some(CollectionResult {
                output_dir: PathBuf::from(".junk-links"),
                link_count: 1,
            }),
        };
        let output = format_run_output(&result);
        assert!(output.contains("COLLECT  1 candidate link(s) in .junk-links"));
    }

    #[test]
    fn directory_hash_output_contains_directory_label() {
        let result = RunResult {
            candidates: vec![candidate(
                CandidateKind::DirectoryHash,
                "backup-a",
                "duplicate of: backup-b\nsha256: digest",
            )],
            collection: None,
        };
        let output = format_run_output(&result);
        assert!(output.contains("DIR_HASH  backup-a"));
        assert!(output.contains("      duplicate of: backup-b"));
        assert!(output.contains("      sha256: digest"));
    }
}
