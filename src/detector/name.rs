use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::candidate::{Candidate, CandidateKind};

const NAME_REASON: &str = "reason: duplicate-like filename";

pub(super) fn detect_by_name(files: &[PathBuf]) -> Vec<Candidate> {
    files
        .iter()
        .filter(|path| has_duplicate_like_file_name(path))
        .map(|path| Candidate {
            path: path.clone(),
            kind: CandidateKind::Name,
            reason: NAME_REASON.to_owned(),
        })
        .collect()
}

fn has_duplicate_like_file_name(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(is_duplicate_like_name)
}

fn is_duplicate_like_name(file_name: &str) -> bool {
    if contains_copy_marker(file_name) {
        return true;
    }
    let stem = file_stem_or_name(file_name);
    contains_parenthesized_number(stem) || ends_with_space_number(stem)
}

fn contains_copy_marker(file_name: &str) -> bool {
    ["コピー", " copy", " Copy"]
        .into_iter()
        .any(|marker| file_name.contains(marker))
}

fn file_stem_or_name(file_name: &str) -> &str {
    Path::new(file_name)
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or(file_name)
}

fn contains_parenthesized_number(value: &str) -> bool {
    value
        .split(" (")
        .skip(1)
        .any(starts_with_number_and_closing_parenthesis)
}

fn starts_with_number_and_closing_parenthesis(value: &str) -> bool {
    let bytes = value.as_bytes();
    let digits = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    digits > 0 && bytes.get(digits) == Some(&b')')
}

fn ends_with_space_number(value: &str) -> bool {
    let Some((prefix, number)) = value.rsplit_once(' ') else {
        return false;
    };
    !prefix.is_empty() && is_number_at_least_two(number)
}

fn is_number_at_least_two(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u64>().is_ok_and(|number| number >= 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_name_cases(names: &[&str], expected: bool) {
        for name in names {
            assert_eq!(is_duplicate_like_name(name), expected, "{name}");
        }
    }

    #[test]
    fn duplicate_like_names_match_requested_patterns() {
        let names = [
            "report のコピー.pdf",
            "reportコピー.pdf",
            "report copy.pdf",
            "report Copy.pdf",
            "report (1).pdf",
            "report (23).pdf",
            "report 2.pdf",
            "report 30.txt",
        ];
        assert_name_cases(&names, true);
    }

    #[test]
    fn ordinary_names_do_not_match() {
        let names = [
            "report.pdf",
            "report (x).pdf",
            "report 1.pdf",
            "copy-report.pdf",
            "report ().pdf",
        ];
        assert_name_cases(&names, false);
    }

    #[test]
    fn detect_by_name_returns_name_candidates() {
        let files = vec![PathBuf::from("report.pdf"), PathBuf::from("report (1).pdf")];
        let candidates = detect_by_name(&files);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].path, files[1]);
        assert_eq!(candidates[0].kind, CandidateKind::Name);
        assert_eq!(candidates[0].reason, NAME_REASON);
    }
}
