mod candidate;
mod collector;
mod config;
mod detector;
mod scanner;

pub use candidate::{Candidate, CandidateKind, CollectionResult, RunResult};
pub use config::RunConfig;

use anyhow::Result;

pub fn run(config: RunConfig) -> Result<RunResult> {
    config.validate()?;

    let scan = scanner::scan_entries_excluding(config.root(), config.output_dir())?;
    let candidates =
        detector::detect_candidates(&scan.files, &scan.directories, config.detection_options())?;
    let collection = config
        .output_dir()
        .map(|output_dir| collector::collect_links(&candidates, output_dir))
        .transpose()?;

    Ok(RunResult::new(candidates, collection))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("hanz-run-{name}-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }

        fn file(&self, relative: &str, content: &str) -> PathBuf {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, content).unwrap();
            path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn run_returns_structured_result_and_creates_links() {
        let root = TestDirectory::new("collect");
        root.file("report (1).pdf", "content");
        let output = root.0.join(".junk-links");
        let config = RunConfig::new(root.0.clone(), true, false, Some(output.clone()));
        let result = run(config).unwrap();
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.collection.as_ref().unwrap().link_count, 1);
        assert!(output.join("report (1).pdf").exists());
        assert!(
            fs::symlink_metadata(output.join("report (1).pdf"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn run_excludes_custom_collection_directory() {
        let root = TestDirectory::new("custom-output");
        let output = root.0.join("collected");
        root.file("collected/old (1).txt", "old");
        let config = RunConfig::new(root.0.clone(), true, false, Some(output.clone()));
        let result = run(config).unwrap();
        assert!(result.candidates.is_empty());
        assert_eq!(result.collection.as_ref().unwrap().link_count, 0);
        assert!(fs::read_dir(output).unwrap().next().is_none());
    }

    #[test]
    fn run_requires_a_detection_mode() {
        let root = TestDirectory::new("mode");
        let config = RunConfig::new(root.0.clone(), false, false, None);
        let error = run(config).unwrap_err().to_string();
        assert_eq!(error, "少なくとも1つの検出方法を有効にしてください");
    }
}
