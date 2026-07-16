mod candidate;
mod config;
mod detector;
mod scanner;

pub use candidate::{Candidate, CandidateKind, RunResult};
pub use config::RunConfig;

use anyhow::Result;

pub fn run(config: RunConfig) -> Result<RunResult> {
    config.validate()?;

    let scan = scanner::scan_entries(config.root())?;
    let candidates =
        detector::detect_candidates(&scan.files, &scan.directories, config.detection_options())?;

    Ok(RunResult::new(candidates))
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
    fn run_requires_a_detection_mode() {
        let root = TestDirectory::new("mode");
        let config = RunConfig::new(root.0.clone(), false, false);
        let error = run(config).unwrap_err().to_string();
        assert_eq!(error, "少なくとも1つの検出方法を有効にしてください");
    }

    #[cfg(unix)]
    #[test]
    fn run_succeeds_without_write_permission() {
        use std::os::unix::fs::PermissionsExt;

        let root = TestDirectory::new("read-only");
        let candidate = root.file("report (1).pdf", "content");
        fs::set_permissions(&candidate, fs::Permissions::from_mode(0o444)).unwrap();
        fs::set_permissions(&root.0, fs::Permissions::from_mode(0o555)).unwrap();

        let result = run(RunConfig::new(root.0.clone(), true, false));

        fs::set_permissions(&root.0, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&candidate, fs::Permissions::from_mode(0o644)).unwrap();

        let result = result.unwrap();
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].path, candidate);
    }
}
