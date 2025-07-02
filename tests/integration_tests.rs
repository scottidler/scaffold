use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper function to create a temporary directory for testing
fn create_temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Helper function to get the scaffold binary command
fn scaffold_cmd() -> Command {
    Command::cargo_bin("scaffold").expect("Failed to find scaffold binary")
}

/// Helper function to check if a generated project builds successfully
fn verify_project_builds(project_dir: &PathBuf) -> bool {
    let output = std::process::Command::new("cargo")
        .args(&["build"])
        .current_dir(project_dir)
        .output()
        .expect("Failed to run cargo build");
    
    output.status.success()
}

/// Helper function to check if a generated project runs successfully
fn verify_project_runs(project_dir: &PathBuf) -> bool {
    let output = std::process::Command::new("cargo")
        .args(&["run", "--", "--help"])
        .current_dir(project_dir)
        .output()
        .expect("Failed to run cargo run");
    
    output.status.success()
}

#[test]
#[serial]
fn test_help_command() {
    scaffold_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("A Rust CLI project scaffolding tool"))
        .stdout(predicate::str::contains("Usage: scaffold"))
        .stdout(predicate::str::contains("PROJECT"))
        .stdout(predicate::str::contains("--author"))
        .stdout(predicate::str::contains("--directory"))
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--no-git"))
        .stdout(predicate::str::contains("--no-sample-config"))
        .stdout(predicate::str::contains("Logs are written to"));
}

#[test]
#[serial]
fn test_version_command() {
    scaffold_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("scaffold"));
}

#[test]
#[serial]
fn test_missing_project_name() {
    scaffold_cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
#[serial]
fn test_invalid_project_name() {
    let temp_dir = create_temp_dir();
    
    scaffold_cmd()
        .arg("invalid@project#name")
        .arg("--directory")
        .arg(temp_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("alphanumeric"));
}

#[test]
#[serial]
fn test_basic_project_creation() {
    let temp_dir = create_temp_dir();
    let project_name = "test-basic";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Creating project"))
        .stdout(predicate::str::contains("Project test-basic created successfully"));
    
    // Verify directory structure
    assert!(project_path.exists());
    assert!(project_path.join("Cargo.toml").exists());
    assert!(project_path.join("build.rs").exists());
    assert!(project_path.join("src").exists());
    assert!(project_path.join("src/main.rs").exists());
    assert!(project_path.join("src/cli.rs").exists());
    assert!(project_path.join("src/config.rs").exists());
    assert!(project_path.join(format!("{}.yml", project_name)).exists());
    assert!(project_path.join(".git").exists());
    
    // Verify project builds
    assert!(verify_project_builds(&project_path), "Generated project should build successfully");
    
    // Verify project runs
    assert!(verify_project_runs(&project_path), "Generated project should run successfully");
}

#[test]
#[serial]
fn test_project_with_custom_author() {
    let temp_dir = create_temp_dir();
    let project_name = "test-author";
    let project_path = temp_dir.path().join(project_name);
    let author = "Custom Author <custom@example.com>";
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .arg("--author")
        .arg(author)
        .assert()
        .success();
    
    // Verify author in Cargo.toml
    let cargo_toml = fs::read_to_string(project_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");
    assert!(cargo_toml.contains(author));
    
    // Verify project builds
    assert!(verify_project_builds(&project_path));
}

#[test]
#[serial]
fn test_project_with_no_git() {
    let temp_dir = create_temp_dir();
    let project_name = "test-no-git";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .arg("--no-git")
        .assert()
        .success();
    
    // Verify no .git directory
    assert!(!project_path.join(".git").exists());
    
    // Verify project still builds
    assert!(verify_project_builds(&project_path));
}

#[test]
#[serial]
fn test_project_with_custom_config() {
    let temp_dir = create_temp_dir();
    let project_name = "test-config";
    let project_path = temp_dir.path().join(project_name);
    
    // Create custom config file
    let config_content = r#"
default_author: "Config Author <config@example.com>"
default_license: "Apache-2.0"
create_git_repo: false
create_sample_config: true
debug: true
template:
  create_build_rs: true
  create_cli_module: true
  create_config_module: true
  dependencies:
    - name: "clap"
      features: ["derive"]
    - name: "eyre"
    - name: "log"
    - name: "env_logger"
    - name: "serde"
      features: ["derive"]
    - name: "serde_yaml"
    - name: "dirs"
    - name: "colored"
  sample_config:
    name: "Config User"
    age: 42
    debug: true
  cli:
    after_help: "Custom help text"
"#;
    
    let config_path = temp_dir.path().join("custom-config.yml");
    fs::write(&config_path, config_content).expect("Failed to write config file");
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .arg("--config")
        .arg(&config_path)
        .assert()
        .success();
    
    // Verify no .git directory (from config)
    assert!(!project_path.join(".git").exists());
    
    // Verify author from config
    let cargo_toml = fs::read_to_string(project_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");
    assert!(cargo_toml.contains("Config Author"));
    
    // Verify project builds
    assert!(verify_project_builds(&project_path));
}

#[test]
#[serial]
fn test_existing_directory_error() {
    let temp_dir = create_temp_dir();
    let project_name = "test-existing";
    let project_path = temp_dir.path().join(project_name);
    
    // Create directory with a file
    fs::create_dir_all(&project_path).expect("Failed to create directory");
    fs::write(project_path.join("existing.txt"), "content").expect("Failed to write file");
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
#[serial]
fn test_generated_cargo_toml_structure() {
    let temp_dir = create_temp_dir();
    let project_name = "test-cargo-toml";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .success();
    
    let cargo_toml = fs::read_to_string(project_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");
    
    // Verify required fields
    assert!(cargo_toml.contains(&format!("name = \"{}\"", project_name)));
    assert!(cargo_toml.contains("version = \"0.1.0\""));
    assert!(cargo_toml.contains("edition = \"2024\""));
    assert!(cargo_toml.contains("build = \"build.rs\""));
    assert!(cargo_toml.contains("[dependencies]"));
    assert!(cargo_toml.contains("[build-dependencies]"));
}

#[test]
#[serial]
fn test_generated_build_rs_structure() {
    let temp_dir = create_temp_dir();
    let project_name = "test-build-rs";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .success();
    
    let build_rs = fs::read_to_string(project_path.join("build.rs"))
        .expect("Failed to read build.rs");
    
    // Verify git describe functionality
    assert!(build_rs.contains("git describe"));
    assert!(build_rs.contains("cargo:rustc-env=GIT_DESCRIBE"));
    assert!(build_rs.contains("cargo:rerun-if-changed=.git/HEAD"));
    assert!(build_rs.contains("cargo:rerun-if-changed=.git/refs/"));
}

#[test]
#[serial]
fn test_generated_main_rs_structure() {
    let temp_dir = create_temp_dir();
    let project_name = "test-main-rs";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .success();
    
    let main_rs = fs::read_to_string(project_path.join("src/main.rs"))
        .expect("Failed to read main.rs");
    
    // Verify required imports
    assert!(main_rs.contains("use clap::Parser"));
    assert!(main_rs.contains("use colored::*"));
    assert!(main_rs.contains("use eyre::{Context, Result}"));
    assert!(main_rs.contains("use log::info"));
    
    // Verify modules
    assert!(main_rs.contains("mod cli"));
    assert!(main_rs.contains("mod config"));
    
    // Verify functions
    assert!(main_rs.contains("fn setup_logging()"));
    assert!(main_rs.contains("fn run_application("));
    assert!(main_rs.contains("fn main()"));
    
    // Verify logging setup
    assert!(main_rs.contains(&format!(".join(\"{}\")", project_name)));
    assert!(main_rs.contains("env_logger::Builder"));
}

#[test]
#[serial]
fn test_generated_cli_rs_structure() {
    let temp_dir = create_temp_dir();
    let project_name = "test-cli-rs";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .success();
    
    let cli_rs = fs::read_to_string(project_path.join("src/cli.rs"))
        .expect("Failed to read cli.rs");
    
    // Verify clap structure
    assert!(cli_rs.contains("use clap::Parser"));
    assert!(cli_rs.contains("#[derive(Parser)]"));
    assert!(cli_rs.contains("#[command("));
    assert!(cli_rs.contains(&format!("name = \"{}\"", project_name)));
    assert!(cli_rs.contains("version = env!(\"GIT_DESCRIBE\")"));
    assert!(cli_rs.contains("pub struct Cli"));
    assert!(cli_rs.contains("pub config: Option<PathBuf>"));
    assert!(cli_rs.contains("pub verbose: bool"));
}

#[test]
#[serial]
fn test_generated_config_rs_structure() {
    let temp_dir = create_temp_dir();
    let project_name = "test-config-rs";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .success();
    
    let config_rs = fs::read_to_string(project_path.join("src/config.rs"))
        .expect("Failed to read config.rs");
    
    // Verify serde structure
    assert!(config_rs.contains("use serde::{Deserialize, Serialize}"));
    assert!(config_rs.contains("#[derive(Debug, Deserialize, Serialize)]"));
    assert!(config_rs.contains("#[serde(default)]"));
    assert!(config_rs.contains("pub struct Config"));
    assert!(config_rs.contains("impl Default for Config"));
    assert!(config_rs.contains("pub fn load("));
    assert!(config_rs.contains("fn load_from_file"));
    
    // Verify config fields
    assert!(config_rs.contains("pub name: String"));
    assert!(config_rs.contains("pub age: u32"));
    assert!(config_rs.contains("pub debug: bool"));
    
    // Verify fallback chain
    assert!(config_rs.contains("dirs::config_dir()"));
    assert!(config_rs.contains("CARGO_PKG_NAME"));
}

#[test]
#[serial]
fn test_generated_sample_config() {
    let temp_dir = create_temp_dir();
    let project_name = "test-sample-config";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .success();
    
    let config_file = project_path.join(format!("{}.yml", project_name));
    assert!(config_file.exists());
    
    let config_content = fs::read_to_string(&config_file)
        .expect("Failed to read config file");
    
    // Verify sample values
    assert!(config_content.contains("name: \"John Doe\""));
    assert!(config_content.contains("age: 30"));
    assert!(config_content.contains("debug: false"));
}

#[test]
#[serial]
fn test_generated_project_dependencies() {
    let temp_dir = create_temp_dir();
    let project_name = "test-dependencies";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .success();
    
    let cargo_toml = fs::read_to_string(project_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");
    
    // Verify all expected dependencies are present
    let expected_deps = ["clap", "eyre", "log", "env_logger", "serde", "serde_yaml", "dirs", "colored"];
    for dep in expected_deps.iter() {
        assert!(cargo_toml.contains(dep), "Missing dependency: {}", dep);
    }
}

#[test]
#[serial]
fn test_generated_project_runs_with_help() {
    let temp_dir = create_temp_dir();
    let project_name = "test-help-output";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .success();
    
    // Test that generated project shows help
    let output = std::process::Command::new("cargo")
        .args(&["run", "--", "--help"])
        .current_dir(&project_path)
        .output()
        .expect("Failed to run generated project");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&project_name));
    assert!(stdout.contains("A CLI application generated by rust-scaffold"));
    assert!(stdout.contains("--config"));
    assert!(stdout.contains("--verbose"));
}

#[test]
#[serial]
fn test_generated_project_runs_with_version() {
    let temp_dir = create_temp_dir();
    let project_name = "test-version-output";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .success();
    
    // Test that generated project shows version
    let output = std::process::Command::new("cargo")
        .args(&["run", "--", "--version"])
        .current_dir(&project_path)
        .output()
        .expect("Failed to run generated project");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&project_name));
}

#[test]
#[serial]
fn test_generated_project_config_loading() {
    let temp_dir = create_temp_dir();
    let project_name = "test-config-loading";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .success();
    
    // Modify the config file
    let config_file = project_path.join(format!("{}.yml", project_name));
    let custom_config = r#"name: "Test User"
age: 42
debug: true"#;
    fs::write(&config_file, custom_config).expect("Failed to write config");
    
    // Run the project and check output
    let output = std::process::Command::new("cargo")
        .args(&["run"])
        .current_dir(&project_path)
        .output()
        .expect("Failed to run generated project");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Test User"));
    assert!(stdout.contains("42"));
    assert!(stdout.contains("Debug mode enabled"));
}

#[test]
#[serial]
fn test_multiple_projects_in_same_session() {
    let temp_dir = create_temp_dir();
    
    // Create multiple projects
    let projects = ["multi-test-1", "multi-test-2", "multi-test-3"];
    
    for project_name in projects.iter() {
        let project_path = temp_dir.path().join(project_name);
        
        scaffold_cmd()
            .arg(project_name)
            .arg("--directory")
            .arg(&project_path)
            .assert()
            .success();
        
        // Verify each project builds
        assert!(verify_project_builds(&project_path), 
                "Project {} should build successfully", project_name);
    }
}

#[test]
#[serial]
fn test_project_name_validation() {
    let temp_dir = create_temp_dir();
    
    // Test valid names
    let valid_names = ["valid-name", "valid_name", "validname123", "a", "test-123"];
    for name in valid_names.iter() {
        let project_path = temp_dir.path().join(name);
        scaffold_cmd()
            .arg(name)
            .arg("--directory")
            .arg(&project_path)
            .assert()
            .success();
    }
    
    // Test invalid names
    let invalid_names = ["invalid@name", "invalid#name", "invalid name", "invalid.name"];
    for name in invalid_names.iter() {
        let project_path = temp_dir.path().join("temp");
        scaffold_cmd()
            .arg(name)
            .arg("--directory")
            .arg(&project_path)
            .assert()
            .failure()
            .stderr(predicate::str::contains("alphanumeric"));
    }
}

#[test]
#[serial]
fn test_logging_setup_in_generated_project() {
    let temp_dir = create_temp_dir();
    let project_name = "test-logging";
    let project_path = temp_dir.path().join(project_name);
    
    scaffold_cmd()
        .arg(project_name)
        .arg("--directory")
        .arg(&project_path)
        .assert()
        .success();
    
    // Run the project to generate logs
    let output = std::process::Command::new("cargo")
        .args(&["run"])
        .current_dir(&project_path)
        .output()
        .expect("Failed to run generated project");
    
    assert!(output.status.success());
    
    // Check that logging directory structure is mentioned in help
    let help_output = std::process::Command::new("cargo")
        .args(&["run", "--", "--help"])
        .current_dir(&project_path)
        .output()
        .expect("Failed to get help output");
    
    let help_text = String::from_utf8_lossy(&help_output.stdout);
    assert!(help_text.contains(&format!("~/.local/share/{}/logs/{}.log", project_name, project_name)));
} 