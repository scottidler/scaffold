#![deny(clippy::unwrap_used)]
#![deny(dead_code)]
#![deny(unused_variables)]

use clap::Parser;
use colored::*;
use eyre::{Context, Result};
use log::{error, info, warn};

use std::fs;
use std::path::PathBuf;
use std::process::Command;

mod cli;
mod config;
mod templates;

use cli::Cli;
use config::Config;

fn setup_logging() -> Result<()> {
    // xdg_data_dir() (not dirs::data_local_dir()) so macOS also honors XDG and logs
    // land in ~/.local/share - matching the path advertised in `--help`.
    let log_dir = config::xdg_data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("scaffold")
        .join("logs");

    fs::create_dir_all(&log_dir).context("Failed to create log directory")?;

    let log_file = log_dir.join("scaffold.log");

    let target = Box::new(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .context("Failed to open log file")?,
    );

    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(target))
        .init();

    info!("Logging initialized, writing to: {}", log_file.display());
    Ok(())
}

/// Check if a directory looks like a fresh cloned repo (only contains typical repo files)
fn is_scaffoldable_directory(dir: &std::path::Path) -> Result<bool> {
    const ALLOWED_FILES: &[&str] = &[
        ".git",
        ".gitignore",
        ".gitattributes",
        ".github",
        ".editorconfig",
        ".vscode",
        ".otto.yml",
        "README",
        "README.md",
        "README.txt",
        "LICENSE",
        "LICENSE.md",
        "LICENSE.txt",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "CODEOWNERS",
        "CONTRIBUTING",
        "CONTRIBUTING.md",
        "CODE_OF_CONDUCT",
        "CODE_OF_CONDUCT.md",
        "SECURITY",
        "SECURITY.md",
    ];

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if !ALLOWED_FILES.iter().any(|&allowed| name.eq_ignore_ascii_case(allowed)) {
            info!("Directory contains non-repo file: {}, blocking scaffold", name);
            return Ok(false);
        }
    }

    Ok(true)
}

/// Resolve the project name and target directory from CLI flags, falling back to CWD.
///
/// - `scaffold` (no args)               -> (basename(CWD), CWD)
/// - `scaffold foo`                     -> ("foo", "./foo")
/// - `scaffold --directory /path/bar`   -> ("bar", "/path/bar")
/// - `scaffold foo --directory /tmp/x`  -> ("foo", "/tmp/x")
fn resolve_project_and_dir(cli: &Cli) -> Result<(String, PathBuf)> {
    // Only read cwd when we actually need it (no positional and no --directory).
    // Reading it unconditionally creates a race with tests that mutate cwd.
    let cwd_needed = cli.project.is_none() && cli.directory.is_none();
    let cwd = if cwd_needed {
        Some(std::env::current_dir().context("Failed to get current directory")?)
    } else {
        None
    };

    let target_dir = match (cli.directory.as_ref(), cli.project.as_ref()) {
        (Some(dir), _) => dir.clone(),
        (None, Some(project)) => PathBuf::from(project),
        (None, None) => cwd.clone().expect("cwd was computed above when both inputs are absent"),
    };

    let project = match cli.project.as_ref() {
        Some(name) => name.clone(),
        None => {
            let absolute = if target_dir.is_absolute() {
                target_dir.clone()
            } else {
                let cwd = std::env::current_dir().context("Failed to get current directory")?;
                cwd.join(&target_dir)
            };
            absolute
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    eyre::eyre!(
                        "Could not derive project name from target directory '{}'; pass it explicitly",
                        target_dir.display()
                    )
                })?
        }
    };

    Ok((project, target_dir))
}

fn create_project(cli: &Cli, config: &Config) -> Result<()> {
    let (project, target_dir) = resolve_project_and_dir(cli)?;

    if project.is_empty() {
        return Err(eyre::eyre!("Project name cannot be empty"));
    }

    if project.starts_with('-') || project.starts_with('_') {
        return Err(eyre::eyre!(
            "Project name cannot start with '-' or '_' (these look like CLI flags)"
        ));
    }

    if project.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return Err(eyre::eyre!(
            "Project name cannot start with a digit (would produce an invalid Rust identifier in the generated lib import)"
        ));
    }

    if !project.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(eyre::eyre!(
            "Project name must contain only alphanumeric characters, hyphens, and underscores"
        ));
    }

    info!("Creating project: {}", project);
    println!("{} Creating project: {}", "✓".green(), project.cyan());

    if target_dir.exists() {
        if !is_scaffoldable_directory(&target_dir)? {
            if cli.force {
                warn!("Directory contains non-repo files, proceeding due to --force");
                println!(
                    "{} Directory contains non-repo files, proceeding due to --force",
                    "⚠".yellow()
                );
            } else {
                return Err(eyre::eyre!(
                    "Directory {} already exists and contains non-repo files (use --force to override)",
                    target_dir.display()
                ));
            }
        }
        println!("{} Using existing directory: {}", "✓".green(), target_dir.display());
    } else {
        fs::create_dir_all(&target_dir).context("Failed to create project directory")?;
        println!("{} Created directory: {}", "✓".green(), target_dir.display());
    }

    templates::generate_project(
        &project,
        &target_dir,
        cli.author.as_ref().unwrap_or(&config.default_author),
        config,
        cli.no_deps,
        cli.force,
        cli.rust_version.as_deref(),
    )?;

    if !cli.no_git && config.create_git_repo {
        init_git_repo(&target_dir)?;
    }

    if !cli.no_deps {
        add_dependencies(&target_dir, config)?;
    }

    if !cli.no_verify {
        verify_build(&target_dir)?;
    }

    println!("\n{} Project {} created successfully!", "🎉".green(), project.cyan());
    println!("Next steps:");
    println!("  cd {}", target_dir.display());
    println!("  cargo run");

    Ok(())
}

fn init_git_repo(target_dir: &PathBuf) -> Result<()> {
    // Skip if .git already exists (e.g., cloned repo)
    if target_dir.join(".git").exists() {
        info!("Git repository already exists, skipping init");
        println!("{} Git repository already exists", "✓".green());
        return Ok(());
    }

    info!("Initializing git repository");
    let output = Command::new("git")
        .args(["init"])
        .current_dir(target_dir)
        .output()
        .context("Failed to run git init")?;

    if output.status.success() {
        println!("{} Initialized git repository", "✓".green());
    } else {
        warn!("Git init failed, continuing without git repository");
        println!("{} Git init failed, continuing without git repository", "⚠".yellow());
    }

    Ok(())
}

fn add_dependencies(target_dir: &PathBuf, config: &Config) -> Result<()> {
    info!("Adding dependencies");
    println!("{} Adding dependencies...", "✓".green());

    for dep in &config.template.dependencies {
        let mut cmd = Command::new("cargo");
        cmd.args(["add", &dep.name]).current_dir(target_dir);

        if !dep.features.is_empty() {
            let features = format!("--features={}", dep.features.join(","));
            cmd.arg(features);
        }

        let output = cmd
            .output()
            .context(format!("Failed to add dependency: {}", dep.name))?;

        if !output.status.success() {
            error!("Failed to add dependency: {}", dep.name);
            return Err(eyre::eyre!("Failed to add dependency: {}", dep.name));
        }
    }

    println!("{} Dependencies added successfully", "✓".green());
    Ok(())
}

fn verify_build(target_dir: &PathBuf) -> Result<()> {
    info!("Verifying project builds");
    println!("{} Verifying project builds...", "✓".green());

    let output = Command::new("cargo")
        .args(["build"])
        .current_dir(target_dir)
        .output()
        .context("Failed to run cargo build")?;

    if output.status.success() {
        println!("{} Project builds successfully", "✓".green());
    } else {
        error!("Project failed to build");
        println!("{}", String::from_utf8_lossy(&output.stderr));
        return Err(eyre::eyre!("Generated project failed to build"));
    }

    Ok(())
}

fn main() -> Result<()> {
    setup_logging().context("Failed to setup logging")?;

    let cli = Cli::parse();

    let config = Config::load(cli.config.as_ref()).context("Failed to load configuration")?;

    info!("Starting scaffold with project name: {:?}", cli.project);

    create_project(&cli, &config).context("Failed to create project")?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_cli(project: &str) -> Cli {
        Cli {
            project: Some(project.to_string()),
            author: Some("Test Author <test@example.com>".to_string()),
            directory: None,
            config: None,
            force: false,
            no_git: true,
            no_sample_config: false,
            no_verify: true,
            no_deps: true,
            rust_version: None,
        }
    }

    fn create_test_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_create_project_validates_empty_name() {
        let cli = Cli {
            project: Some("".to_string()),
            author: None,
            directory: None,
            config: None,
            force: false,
            no_git: true,
            no_sample_config: false,
            no_verify: true,
            no_deps: true,
            rust_version: None,
        };
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Project name cannot be empty"));
    }

    #[test]
    fn test_create_project_validates_name_starting_with_dash() {
        let cli = Cli {
            project: Some("-invalid".to_string()),
            author: None,
            directory: None,
            config: None,
            force: false,
            no_git: true,
            no_sample_config: false,
            no_verify: true,
            no_deps: true,
            rust_version: None,
        };
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot start with '-' or '_'"));
    }

    #[test]
    fn test_create_project_validates_name_starting_with_underscore() {
        let cli = Cli {
            project: Some("_invalid".to_string()),
            author: None,
            directory: None,
            config: None,
            force: false,
            no_git: true,
            no_sample_config: false,
            no_verify: true,
            no_deps: true,
            rust_version: None,
        };
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot start with '-' or '_'"));
    }

    #[test]
    fn test_create_project_validates_invalid_characters() {
        let cli = Cli {
            project: Some("invalid@name".to_string()),
            author: None,
            directory: None,
            config: None,
            force: false,
            no_git: true,
            no_sample_config: false,
            no_verify: true,
            no_deps: true,
            rust_version: None,
        };
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must contain only alphanumeric characters")
        );
    }

    #[test]
    fn test_create_project_accepts_valid_names() {
        let temp_dir = TempDir::new().unwrap();
        let valid_names = ["valid", "valid-name", "valid_name", "valid123", "v"];

        for name in valid_names.iter() {
            let project_dir = temp_dir.path().join(name);
            let mut cli = create_test_cli(name);
            cli.directory = Some(project_dir.clone());
            let config = create_test_config();

            let result = create_project(&cli, &config);
            assert!(result.is_ok(), "Failed for valid name: {}", name);
            assert!(project_dir.exists());
        }
    }

    #[test]
    fn test_create_project_fails_on_directory_with_non_repo_files() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-project");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(project_dir.join("existing.txt"), "content").unwrap();

        let mut cli = create_test_cli("test-project");
        cli.directory = Some(project_dir);
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("contains non-repo files"));
    }

    #[test]
    fn test_create_project_succeeds_on_directory_with_repo_files_only() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-project-repo");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(project_dir.join("README.md"), "# Test").unwrap();
        fs::write(project_dir.join("LICENSE"), "MIT").unwrap();
        fs::write(project_dir.join(".gitignore"), "/target").unwrap();

        let mut cli = create_test_cli("test-project-repo");
        cli.directory = Some(project_dir.clone());
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_ok());
        assert!(project_dir.join("Cargo.toml").exists());
        assert!(project_dir.join("src").exists());
        // Original files should still exist
        assert!(project_dir.join("README.md").exists());
        assert!(project_dir.join("LICENSE").exists());
        assert!(project_dir.join(".gitignore").exists());
    }

    #[test]
    fn test_create_project_succeeds_on_directory_with_git_folder() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-project-git");
        fs::create_dir_all(project_dir.join(".git")).unwrap();
        fs::write(project_dir.join("README.md"), "# Test").unwrap();

        let mut cli = create_test_cli("test-project-git");
        cli.directory = Some(project_dir.clone());
        cli.no_git = false; // Enable git to test skipping
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_ok());
        assert!(project_dir.join("Cargo.toml").exists());
        // .git should still exist (not re-initialized)
        assert!(project_dir.join(".git").exists());
    }

    #[test]
    fn test_create_project_succeeds_on_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-project");
        fs::create_dir_all(&project_dir).unwrap();

        let mut cli = create_test_cli("test-project");
        cli.directory = Some(project_dir.clone());
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_ok());
        assert!(project_dir.join("Cargo.toml").exists());
        assert!(project_dir.join("src").exists());
    }

    #[test]
    #[serial_test::serial(cwd)]
    fn test_create_project_uses_default_directory() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let cli = create_test_cli("test-default");
        let config = create_test_config();

        let result = create_project(&cli, &config);

        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        assert!(temp_dir.path().join("test-default").exists());
    }

    #[test]
    fn test_create_project_uses_custom_author() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-author");

        let mut cli = create_test_cli("test-author");
        cli.directory = Some(project_dir.clone());
        cli.author = Some("Custom Author <custom@test.com>".to_string());
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_ok());

        let cargo_toml = fs::read_to_string(project_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("Custom Author <custom@test.com>"));
    }

    #[test]
    fn test_create_project_uses_config_author_as_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-config-author");

        let mut cli = create_test_cli("test-config-author");
        cli.directory = Some(project_dir.clone());
        cli.author = None; // No author specified
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_ok());

        let cargo_toml = fs::read_to_string(project_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains(&config.default_author));
    }

    #[test]
    #[serial_test::serial(cwd)]
    fn test_create_project_derives_name_from_cwd_when_unspecified() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("from-cwd");
        fs::create_dir_all(&project_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_dir).unwrap();

        let mut cli = create_test_cli("placeholder");
        cli.project = None;
        cli.directory = None;
        let config = create_test_config();

        let result = create_project(&cli, &config);

        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        assert!(project_dir.join("Cargo.toml").exists());
        let cargo_toml = fs::read_to_string(project_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("name = \"from-cwd\""));
    }

    #[test]
    fn test_create_project_derives_name_from_directory_flag() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("from-flag");

        let mut cli = create_test_cli("placeholder");
        cli.project = None;
        cli.directory = Some(project_dir.clone());
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_ok());
        let cargo_toml = fs::read_to_string(project_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("name = \"from-flag\""));
    }

    #[test]
    fn test_create_project_emits_lib_rs_and_tests_rs() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-lib-bin");

        let mut cli = create_test_cli("test-lib-bin");
        cli.directory = Some(project_dir.clone());
        let config = create_test_config();

        create_project(&cli, &config).unwrap();

        assert!(project_dir.join("src/lib.rs").exists(), "src/lib.rs must be generated");
        assert!(
            project_dir.join("src/tests.rs").exists(),
            "src/tests.rs must be generated"
        );
        assert!(project_dir.join("src/main.rs").exists());
        assert!(project_dir.join("src/cli.rs").exists());
        assert!(project_dir.join("src/config.rs").exists());
    }

    #[test]
    fn test_generated_main_uses_library() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("test-uses-lib");

        let mut cli = create_test_cli("test-uses-lib");
        cli.directory = Some(project_dir.clone());
        let config = create_test_config();

        create_project(&cli, &config).unwrap();

        let main_rs = fs::read_to_string(project_dir.join("src/main.rs")).unwrap();
        assert!(
            main_rs.contains("use test_uses_lib::{Config, run};"),
            "generated main.rs must import Config and run from the library crate"
        );
        assert!(main_rs.contains("mod cli;"), "generated main.rs still declares mod cli");
        assert!(
            !main_rs.contains("mod config;"),
            "generated main.rs must not declare mod config (moved to lib.rs)"
        );
        assert!(
            !main_rs.contains("fn run_application"),
            "generated main.rs must not contain run_application (moved to lib::run)"
        );
    }

    #[test]
    fn test_generated_main_translates_hyphens() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("foo-bar");

        let mut cli = create_test_cli("foo-bar");
        cli.directory = Some(project_dir.clone());
        let config = create_test_config();

        create_project(&cli, &config).unwrap();

        let main_rs = fs::read_to_string(project_dir.join("src/main.rs")).unwrap();
        assert!(
            main_rs.contains("use foo_bar::{Config, run};"),
            "hyphens in package name must translate to underscores in the use statement"
        );
        assert!(
            !main_rs.contains("use foo-bar::"),
            "hyphenated form must not appear in the use statement (would not compile)"
        );
    }

    #[test]
    fn test_create_project_rejects_leading_digit_name() {
        let cli = Cli {
            project: Some("1pass".to_string()),
            author: None,
            directory: None,
            config: None,
            force: false,
            no_git: true,
            no_sample_config: false,
            no_verify: true,
            no_deps: true,
            rust_version: None,
        };
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("cannot start with a digit"),
            "leading-digit names must be rejected with a clear error"
        );
    }
}
