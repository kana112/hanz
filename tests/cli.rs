use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after Unix epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("hanz-cli-{name}-{}-{nonce}", std::process::id()));
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

fn run_cli(args: &[&Path]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_hanz"))
        .args(args)
        .output()
        .expect("hanz should start")
}

#[test]
fn name_detection_preserves_output_format() {
    let root = TestDirectory::new("name");
    let file = root.file("report (1).pdf", "content");
    let output = Command::new(env!("CARGO_BIN_EXE_hanz"))
        .arg(&root.0)
        .arg("--name")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout,
        format!(
            "NAME  {}\n      reason: duplicate-like filename\n\n",
            file.display()
        )
    );
}

#[test]
fn hash_detection_finds_identical_files() {
    let root = TestDirectory::new("hash");
    root.file("a.txt", "same");
    root.file("b.txt", "same");
    let output = Command::new(env!("CARGO_BIN_EXE_hanz"))
        .arg(&root.0)
        .arg("--hash")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("HASH  "));
    assert!(stdout.contains("duplicate of:"));
    assert!(stdout.contains("sha256: "));
}

#[test]
fn collect_creates_candidate_links() {
    let root = TestDirectory::new("collect");
    let source = root.file("report (1).pdf", "content");
    let output_dir = root.0.join("collected");
    let output = Command::new(env!("CARGO_BIN_EXE_hanz"))
        .arg(&root.0)
        .arg("--name")
        .arg("--collect")
        .arg(&output_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("COLLECT  1 candidate link(s)"));
    let link = output_dir.join("report (1).pdf");
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::canonicalize(link).unwrap(),
        fs::canonicalize(source).unwrap()
    );
}

#[test]
fn missing_detection_mode_is_a_cli_error() {
    let root = TestDirectory::new("missing-mode");
    let output = run_cli(&[root.0.as_path()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--name"));
    assert!(stderr.contains("--hash"));
}

#[test]
fn completions_option_generates_shell_files() {
    let root = TestDirectory::new("completions");
    let output = Command::new(env!("CARGO_BIN_EXE_hanz"))
        .current_dir(&root.0)
        .arg("--completions")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(root.0.join("completions/bash/hanz").is_file());
    assert!(root.0.join("completions/zsh/_hanz").is_file());
    assert!(root.0.join("completions/fish/hanz").is_file());
    let bash = fs::read_to_string(root.0.join("completions/bash/hanz")).unwrap();
    assert!(bash.contains("--name"));
    assert!(bash.contains("--hash"));
    assert!(bash.contains("--collect"));
}
