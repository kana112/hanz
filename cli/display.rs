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
    format_candidates(&result.candidates)
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
        };
        let output = format_run_output(&result);
        assert!(output.contains("NAME  report (1).pdf"));
        assert!(output.contains("      reason: duplicate-like filename"));
    }

    #[test]
    fn directory_hash_output_contains_directory_label() {
        let result = RunResult {
            candidates: vec![candidate(
                CandidateKind::DirectoryHash,
                "backup-a",
                "duplicate of: backup-b\nsha256: digest",
            )],
        };
        let output = format_run_output(&result);
        assert!(output.contains("DIR_HASH  backup-a"));
        assert!(output.contains("      duplicate of: backup-b"));
        assert!(output.contains("      sha256: digest"));
    }
}
