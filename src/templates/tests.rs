#![allow(clippy::unwrap_used)]
use super::*;
use std::fs;
use tempfile::TempDir;

fn create_test_config() -> Config {
    Config::default()
}

#[test]
fn test_generate_project_creates_all_files() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "test-project";
    let author = "Test Author <test@example.com>";
    let config = create_test_config();

    let result = generate_project(
        project_name,
        temp_dir.path(),
        author,
        &config,
        false,
        false,
        None,
        "scottidler",
        "v1",
    );
    assert!(result.is_ok());

    // Check that all expected files were created
    assert!(temp_dir.path().join("Cargo.toml").exists());
    assert!(temp_dir.path().join("build.rs").exists());
    assert!(temp_dir.path().join("src").exists());
    assert!(temp_dir.path().join("src/main.rs").exists());
    assert!(temp_dir.path().join("src/cli.rs").exists());
    assert!(temp_dir.path().join("src/config.rs").exists());
    assert!(temp_dir.path().join("src/config/tests.rs").exists());
    assert!(temp_dir.path().join(format!("{}.yml", project_name)).exists());
    assert!(temp_dir.path().join(".github/workflows/ci.yml").exists());
    assert!(temp_dir.path().join(".github/workflows/release.yml").exists());
    assert!(temp_dir.path().join(".pre-commit-config.yaml").exists());
    assert!(temp_dir.path().join(".github/workflows/mutants.yml").exists());
}

#[test]
fn test_generate_cargo_toml_basic_structure() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "test-cargo";
    let author = "Test Author <test@example.com>";
    let config = create_test_config();

    let result = generate_cargo_toml(project_name, temp_dir.path(), author, &config, false, false);
    assert!(result.is_ok());

    let cargo_toml = fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();

    assert!(cargo_toml.contains(&format!("name = \"{}\"", project_name)));
    assert!(cargo_toml.contains("version = \"0.1.0\""));
    assert!(cargo_toml.contains("edition = \"2024\""));
    assert!(cargo_toml.contains(author));
    assert!(cargo_toml.contains("build = \"build.rs\""));
    assert!(cargo_toml.contains("[dependencies]"));
    assert!(cargo_toml.contains("[dev-dependencies]"));
    assert!(cargo_toml.contains("tempfile = \"3\""));
    assert!(cargo_toml.contains("[build-dependencies]"));
}

#[test]
fn test_generate_cargo_toml_with_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "test-deps";
    let author = "Test Author";
    let config = create_test_config();

    let result = generate_cargo_toml(project_name, temp_dir.path(), author, &config, true, false);
    assert!(result.is_ok());

    let cargo_toml = fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();

    // Check that dependencies are included
    assert!(cargo_toml.contains("clap"));
    assert!(cargo_toml.contains("eyre"));
    assert!(cargo_toml.contains("serde"));
    assert!(cargo_toml.contains("derive"));
}

#[test]
fn test_generate_cargo_toml_without_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "test-no-deps";
    let author = "Test Author";
    let config = create_test_config();

    let result = generate_cargo_toml(project_name, temp_dir.path(), author, &config, false, false);
    assert!(result.is_ok());

    let cargo_toml = fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();

    // Check that dependencies comment is included instead
    assert!(cargo_toml.contains("Dependencies will be added via cargo add"));
    assert!(!cargo_toml.contains("clap = "));
    assert!(!cargo_toml.contains("eyre = "));
}

#[test]
fn test_generate_build_rs() {
    let temp_dir = TempDir::new().unwrap();

    let result = generate_build_rs(temp_dir.path(), false);
    assert!(result.is_ok());

    let build_rs = fs::read_to_string(temp_dir.path().join("build.rs")).unwrap();

    assert!(build_rs.contains("fn main()"));
    assert!(build_rs.contains("git describe"));
    assert!(build_rs.contains("GIT_DESCRIBE"));
    assert!(build_rs.contains("cargo:rerun-if-changed=.git/HEAD"));
    assert!(build_rs.contains("cargo:rerun-if-changed=.git/refs/"));
}

#[test]
fn test_generate_main_rs() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let project_name = "test-main";

    let result = generate_main_rs(project_name, &src_dir, false);
    assert!(result.is_ok());

    let main_rs = fs::read_to_string(src_dir.join("main.rs")).unwrap();

    assert!(main_rs.contains("use clap::Parser"));
    assert!(main_rs.contains("use colored::*"));
    assert!(main_rs.contains("use eyre::{Context, Result}"));
    assert!(main_rs.contains("fn main()"));
    assert!(main_rs.contains("fn setup_logging()"));
    assert!(main_rs.contains("xdg_data_dir()"));
    assert!(
        !main_rs.contains("dirs::data_local_dir()"),
        "generated main.rs must use xdg_data_dir(), not dirs::data_local_dir() (wrong path on macOS)"
    );
    assert!(main_rs.contains("use test_main::{Config, run};"));
    assert!(main_rs.contains("let result = run(&config)"));
    assert!(main_rs.contains(project_name));
    assert!(main_rs.contains("mod cli"));
    assert!(!main_rs.contains("mod config"));
    assert!(!main_rs.contains("fn run_application"));
}

#[test]
fn test_generate_main_rs_translates_hyphens_to_underscores() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let project_name = "foo-bar-baz";

    generate_main_rs(project_name, &src_dir, false).unwrap();

    let main_rs = fs::read_to_string(src_dir.join("main.rs")).unwrap();

    assert!(main_rs.contains("use foo_bar_baz::{Config, run};"));
    assert!(!main_rs.contains("use foo-bar-baz::"));
}

#[test]
fn test_generate_lib_rs() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    generate_lib_rs(&src_dir, false).unwrap();

    let lib_rs = fs::read_to_string(src_dir.join("lib.rs")).unwrap();

    assert!(lib_rs.contains("#![deny(clippy::unwrap_used)]"));
    assert!(lib_rs.contains("pub mod config;"));
    assert!(lib_rs.contains("pub use config::Config;"));
    assert!(lib_rs.contains("pub struct RunResult"));
    assert!(lib_rs.contains("pub fn run(config: &Config) -> Result<RunResult>"));
    assert!(lib_rs.contains("#[cfg(test)]"));
    assert!(lib_rs.contains("mod tests;"));
    // The library must not depend on clap; only the binary parses CLI args.
    assert!(!lib_rs.contains("use clap"));
    assert!(!lib_rs.contains("pub mod cli"));
}

#[test]
fn test_generate_lib_tests_rs() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    generate_lib_tests_rs(&src_dir, false).unwrap();

    let tests_rs = fs::read_to_string(src_dir.join("tests.rs")).unwrap();

    assert!(tests_rs.contains("use super::*;"));
    assert!(tests_rs.contains("test_run_returns_messages"));
    assert!(tests_rs.contains("test_run_messages_mention_project_name"));
}

#[test]
fn test_generate_cli_rs() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let project_name = "test-cli";
    let config = create_test_config();

    let result = generate_cli_rs(project_name, &src_dir, &config, false);
    assert!(result.is_ok());

    let cli_rs = fs::read_to_string(src_dir.join("cli.rs")).unwrap();

    assert!(cli_rs.contains("use clap::Parser"));
    assert!(cli_rs.contains("#[derive(Parser)]"));
    assert!(cli_rs.contains(&format!("name = \"{}\"", project_name)));
    assert!(cli_rs.contains("pub struct Cli"));
    assert!(cli_rs.contains("pub config: Option<PathBuf>"));
    assert!(cli_rs.contains("pub verbose: bool"));
    assert!(cli_rs.contains("A CLI application generated by rust-scaffold"));
    // after_help advertises the XDG log path, with {{PROJECT}} substituted.
    assert!(cli_rs.contains("after_help ="));
    assert!(cli_rs.contains(&format!("logs/{}.log", project_name)));
    assert!(!cli_rs.contains("{{PROJECT}}"));
}

#[test]
fn test_generate_config_rs() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let result = generate_config_rs(&src_dir, false);
    assert!(result.is_ok());

    let config_rs = fs::read_to_string(src_dir.join("config.rs")).unwrap();

    assert!(config_rs.contains("use serde::{Deserialize, Serialize}"));
    assert!(config_rs.contains("rename_all = \"kebab-case\""));
    assert!(config_rs.contains("pub struct Config"));
    assert!(config_rs.contains("pub name: String"));
    assert!(config_rs.contains("pub age: u32"));
    assert!(config_rs.contains("pub debug: bool"));
    assert!(config_rs.contains("impl Default for Config"));
    assert!(config_rs.contains("fn xdg_config_dir()"));
    assert!(config_rs.contains("pub fn xdg_data_dir()"));
    assert!(
        !config_rs.contains("dirs::config_dir()"),
        "generated config.rs must use xdg_config_dir(), not dirs::config_dir() (wrong path on macOS)"
    );
    assert!(config_rs.contains("pub fn load"));
    assert!(config_rs.contains("fn load_from_file"));
    assert!(config_rs.contains("John Doe"));
    assert!(config_rs.contains("#[cfg(test)]\nmod tests;"));
}

#[test]
fn test_generate_config_tests_rs() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let result = generate_config_tests_rs(&src_dir, false);
    assert!(result.is_ok());

    let tests_rs = fs::read_to_string(src_dir.join("config/tests.rs")).unwrap();

    assert!(tests_rs.contains("use super::*"));
    assert!(tests_rs.contains("XDG_CONFIG_HOME"));
    assert!(tests_rs.contains("XDG_DATA_HOME"));
    assert!(tests_rs.contains("test_xdg_config_dir_honors_env_and_falls_back"));
    assert!(tests_rs.contains("test_xdg_data_dir_honors_env_and_falls_back"));
    // The macOS Application Support assertion is gone - paths are XDG on every platform.
    assert!(
        !tests_rs.contains("Application Support"),
        "generated tests must not assert the macOS Application Support path"
    );
    assert!(tests_rs.contains("test_config_load_from_explicit_path"));
    assert!(tests_rs.contains("test_config_default_values"));
}

#[test]
fn test_generate_sample_config() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "test-sample";

    let result = generate_sample_config(project_name, temp_dir.path(), false);
    assert!(result.is_ok());

    let config_file = temp_dir.path().join(format!("{}.yml", project_name));
    assert!(config_file.exists());

    let config_content = fs::read_to_string(&config_file).unwrap();

    assert!(config_content.contains(project_name));
    assert!(config_content.contains("name: John Doe"));
    assert!(config_content.contains("age: 30"));
    assert!(config_content.contains("debug: false"));
    assert!(config_content.contains("Sample configuration file"));
}

#[test]
fn test_generate_project_with_different_authors() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "test-authors";
    let config = create_test_config();

    let authors = [
        "Simple Name",
        "Name With Spaces <email@example.com>",
        "Unicode Äuthör <ünicöde@example.com>",
    ];

    for author in authors.iter() {
        let project_dir = temp_dir.path().join(author.replace(['<', '>', '@', ' '], "_"));
        let result = generate_project(
            project_name,
            &project_dir,
            author,
            &config,
            false,
            false,
            None,
            "scottidler",
            "v1",
        );
        assert!(result.is_ok(), "Failed for author: {}", author);

        let cargo_toml = fs::read_to_string(project_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains(author));
    }
}

#[test]
fn test_generate_project_with_special_characters_in_name() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config();
    let author = "Test Author";

    let project_names = ["simple", "with-dashes", "with_underscores", "with123numbers"];

    for project_name in project_names.iter() {
        let project_dir = temp_dir.path().join(project_name);
        let result = generate_project(
            project_name,
            &project_dir,
            author,
            &config,
            false,
            false,
            None,
            "scottidler",
            "v1",
        );
        assert!(result.is_ok(), "Failed for project name: {}", project_name);

        let cargo_toml = fs::read_to_string(project_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains(&format!("name = \"{}\"", project_name)));

        let main_rs = fs::read_to_string(project_dir.join("src/main.rs")).unwrap();
        assert!(main_rs.contains(project_name));

        let cli_rs = fs::read_to_string(project_dir.join("src/cli.rs")).unwrap();
        assert!(cli_rs.contains(&format!("name = \"{}\"", project_name)));

        let sample_config = project_dir.join(format!("{}.yml", project_name));
        assert!(sample_config.exists());
    }
}

#[test]
fn test_detect_rust_version_returns_dotted_semver() {
    // rustc is always present in this project's build/test environment, so
    // detection should yield a dotted version (e.g. "1.96.0"), never the
    // bare fallback placeholder.
    let version = detect_rust_version();
    assert!(!version.is_empty());
    assert!(
        version.split('.').count() >= 2,
        "expected a dotted version, got: {}",
        version
    );
    assert!(version.chars().next().unwrap().is_ascii_digit());
}

#[test]
// `detect_rust_version` spawns `rustc` in the process cwd, which the `serial(cwd)` tests in main.rs
// point at a TempDir they then delete; run outside that window or rustc fails and the pin falls back.
#[serial_test::serial(cwd)]
fn test_generate_project_pins_detected_version_without_override() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config();
    let project_dir = temp_dir.path().join("detected");

    let result = generate_project(
        "detected",
        &project_dir,
        "Test Author",
        &config,
        false,
        false,
        None,
        "scottidler",
        "v1",
    );
    assert!(result.is_ok());

    let ci_yml = fs::read_to_string(project_dir.join(".github/workflows/ci.yml")).unwrap();
    assert!(ci_yml.contains(&format!("rust-version: \"{}\"", detect_rust_version())));
    assert!(!ci_yml.contains("__RUST_VERSION__"));
}

#[test]
fn test_generate_github_ci_yml() {
    let temp_dir = TempDir::new().unwrap();
    let workflows_dir = temp_dir.path().join(".github").join("workflows");
    fs::create_dir_all(&workflows_dir).unwrap();

    let result = generate_github_ci_yml(&workflows_dir, "1.95.0", "scottidler", "v1", false);
    assert!(result.is_ok());

    let ci_yml = fs::read_to_string(workflows_dir.join("ci.yml")).unwrap();

    assert!(ci_yml.contains("name: CI"));
    assert!(ci_yml.contains("branches: [main]"));
    // Thin caller: wires to the org's reusable rust-ci workflow at the given ref,
    // passing the pinned toolchain through as the rust-version input.
    assert!(ci_yml.contains("uses: scottidler/github-actions/.github/workflows/rust-ci.yml@v1"));
    assert!(ci_yml.contains("rust-version: \"1.95.0\""));
    assert!(!ci_yml.contains("__RUST_VERSION__"));
    assert!(!ci_yml.contains("__ORG__"));
    assert!(!ci_yml.contains("__REF__"));
    assert!(ci_yml.contains("permissions:"));
    assert!(ci_yml.contains("contents: read"));
    // The pipeline now lives in the reusable workflow, not inlined here.
    assert!(!ci_yml.contains("cargo clippy"));
}

#[test]
fn test_generate_github_ci_yml_honors_org_and_ref() {
    let temp_dir = TempDir::new().unwrap();
    let workflows_dir = temp_dir.path().join(".github").join("workflows");
    fs::create_dir_all(&workflows_dir).unwrap();

    let result = generate_github_ci_yml(&workflows_dir, "1.96.0", "tatari-tv", "v5", false);
    assert!(result.is_ok());

    let ci_yml = fs::read_to_string(workflows_dir.join("ci.yml")).unwrap();
    assert!(ci_yml.contains("uses: tatari-tv/github-actions/.github/workflows/rust-ci.yml@v5"));
}

#[test]
fn test_generate_github_release_yml() {
    let temp_dir = TempDir::new().unwrap();
    let workflows_dir = temp_dir.path().join(".github").join("workflows");
    fs::create_dir_all(&workflows_dir).unwrap();

    let result = generate_github_release_yml(&workflows_dir, "1.95.0", "scottidler", "v1", false);
    assert!(result.is_ok());

    let release_yml = fs::read_to_string(workflows_dir.join("release.yml")).unwrap();

    assert!(release_yml.contains("name: Release"));
    assert!(release_yml.contains("- 'v*'"));
    // Thin caller: wires to the org's reusable rust-cli-release workflow at the ref,
    // passing the pinned toolchain through as the rust-version input.
    assert!(release_yml.contains("uses: scottidler/github-actions/.github/workflows/rust-cli-release.yml@v1"));
    assert!(release_yml.contains("rust-version: \"1.95.0\""));
    assert!(!release_yml.contains("__RUST_VERSION__"));
    assert!(!release_yml.contains("__ORG__"));
    assert!(!release_yml.contains("__REF__"));
    assert!(release_yml.contains("contents: write"));
    // The cross-compile matrix now lives in the reusable workflow, not inlined here.
    assert!(!release_yml.contains("x86_64-unknown-linux-gnu"));
    assert!(!release_yml.contains("softprops/action-gh-release"));
    // No leftover template placeholder.
    assert!(!release_yml.contains("{{PROJECT}}"));
    // Verify no Docker job
    assert!(!release_yml.contains("docker"));
    assert!(!release_yml.contains("Dockerfile"));
}

#[test]
fn test_generate_github_release_yml_honors_org_and_ref() {
    let temp_dir = TempDir::new().unwrap();
    let workflows_dir = temp_dir.path().join(".github").join("workflows");
    fs::create_dir_all(&workflows_dir).unwrap();

    let result = generate_github_release_yml(&workflows_dir, "1.96.0", "tatari-tv", "v5", false);
    assert!(result.is_ok());

    let release_yml = fs::read_to_string(workflows_dir.join("release.yml")).unwrap();
    assert!(release_yml.contains("uses: tatari-tv/github-actions/.github/workflows/rust-cli-release.yml@v5"));
}

#[test]
fn test_generate_github_workflows() {
    let temp_dir = TempDir::new().unwrap();

    let result = generate_github_workflows(temp_dir.path(), "1.95.0", "scottidler", "v1", false);
    assert!(result.is_ok());

    assert!(temp_dir.path().join(".github/workflows/ci.yml").exists());
    assert!(temp_dir.path().join(".github/workflows/release.yml").exists());
}

#[test]
fn test_generate_project_creates_src_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "test-src";
    let author = "Test Author";
    let config = create_test_config();

    // Ensure src directory doesn't exist initially
    assert!(!temp_dir.path().join("src").exists());

    let result = generate_project(
        project_name,
        temp_dir.path(),
        author,
        &config,
        false,
        false,
        None,
        "scottidler",
        "v1",
    );
    assert!(result.is_ok());

    // Verify src directory was created
    assert!(temp_dir.path().join("src").exists());
    assert!(temp_dir.path().join("src").is_dir());
}

#[test]
fn test_generate_github_mutants_yml_pins_public_reusable_workflow() {
    let temp_dir = TempDir::new().unwrap();
    generate_github_mutants_yml(temp_dir.path(), "1.99.1", false).unwrap();

    let yml = fs::read_to_string(temp_dir.path().join("mutants.yml")).unwrap();
    assert!(yml.contains("uses: tatari-tv/github-actions-public/.github/workflows/mutants.yml@v2.3.1"));
    assert!(yml.contains("rust-version: \"1.99.1\""));
    assert!(!yml.contains("__RUST_VERSION__"));
    assert!(yml.contains("pull-requests: write"));
}

#[test]
fn test_generate_github_workflows_emits_mutants_caller() {
    let temp_dir = TempDir::new().unwrap();
    generate_github_workflows(temp_dir.path(), "1.95.0", "scottidler", "v1", false).unwrap();
    assert!(temp_dir.path().join(".github/workflows/mutants.yml").exists());
}

#[test]
fn test_generate_otto_yml_has_mutants_task_not_in_ci() {
    let temp_dir = TempDir::new().unwrap();
    generate_otto_yml("my-proj", temp_dir.path(), false).unwrap();

    let otto = fs::read_to_string(temp_dir.path().join(".otto.yml")).unwrap();
    assert!(otto.contains("\n  mutants:\n"));
    assert!(otto.contains("--in-place"));
    assert!(otto.contains("'^+++ b/'"));
    assert!(otto.contains("/var/tmp/my-proj-mutants"));
    assert!(!otto.contains("__PROJECT__"));
    assert!(otto.contains("before: [lint, bloat, check, test]"));
}

#[test]
fn test_generate_gitignore_creates_with_entries() {
    let temp_dir = TempDir::new().unwrap();
    generate_gitignore(temp_dir.path()).unwrap();

    let gi = fs::read_to_string(temp_dir.path().join(".gitignore")).unwrap();
    assert!(gi.lines().any(|l| l == "/rc"));
    assert!(gi.lines().any(|l| l == "mutants.out*"));
}

#[test]
fn test_generate_gitignore_appends_missing_and_is_idempotent() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join(".gitignore");
    fs::write(&path, "/target\n*.swp").unwrap();

    generate_gitignore(temp_dir.path()).unwrap();
    generate_gitignore(temp_dir.path()).unwrap();

    let gi = fs::read_to_string(&path).unwrap();
    assert!(gi.contains("*.swp\n"));
    assert_eq!(gi.lines().filter(|l| *l == "/rc").count(), 1);
    assert_eq!(gi.lines().filter(|l| *l == "/target").count(), 1);
}

#[test]
fn test_generate_otto_yml_mutants_features_match_otto_test() {
    let temp_dir = TempDir::new().unwrap();
    generate_otto_yml("my-proj", temp_dir.path(), false).unwrap();

    let otto = fs::read_to_string(temp_dir.path().join(".otto.yml")).unwrap();
    let test_line = otto.lines().find(|l| l.trim_start().starts_with("cargo test")).unwrap();
    let mutants_line = otto
        .lines()
        .find(|l| l.trim_start().starts_with("cargo mutants --workspace"))
        .unwrap();
    assert_eq!(
        test_line.contains("--all-features"),
        mutants_line.contains("--all-features")
    );
}

#[test]
fn test_generate_otto_yml_mutants_guard_reads_diff_once_and_checks_every_header() {
    let temp_dir = TempDir::new().unwrap();
    generate_otto_yml("my-proj", temp_dir.path(), false).unwrap();

    let otto = fs::read_to_string(temp_dir.path().join(".otto.yml")).unwrap();
    assert!(otto.contains("diff_copy=$(mktemp)"));
    assert!(otto.contains("--in-diff \"$diff_copy\""));
    assert!(otto.contains("grep -v -e '^+++ b/' -e '^+++ /dev/null'"));
    assert!(!otto.contains("[ -s \"${diff}\" ]"));
    assert!(otto.contains("awk '{print $2}' || true)"));
    assert!(otto.contains("trap 'rm -f \"$diff_copy\"' EXIT"));
    assert!(otto.contains("! grep -q '^+++ ' \"$diff_copy\""));
}
