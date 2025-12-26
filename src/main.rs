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
    let log_dir = dirs::data_local_dir()
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

fn create_project(cli: &Cli, config: &Config) -> Result<()> {
    let project = &cli.project;
    let default_dir = PathBuf::from(project);
    let target_dir = cli.directory.as_ref().unwrap_or(&default_dir);

    if project.is_empty() {
        return Err(eyre::eyre!("Project name cannot be empty"));
    }

    if project.starts_with('-') || project.starts_with('_') {
        return Err(eyre::eyre!(
            "Project name cannot start with '-' or '_' (these look like CLI flags)"
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
        if !is_scaffoldable_directory(target_dir)? {
            return Err(eyre::eyre!(
                "Directory {} already exists and contains non-repo files",
                target_dir.display()
            ));
        }
        println!("{} Using existing directory: {}", "✓".green(), target_dir.display());
    } else {
        fs::create_dir_all(target_dir).context("Failed to create project directory")?;
        println!("{} Created directory: {}", "✓".green(), target_dir.display());
    }

    templates::generate_project(
        project,
        target_dir,
        cli.author.as_ref().unwrap_or(&config.default_author),
        config,
        cli.no_deps,
    )?;

    if !cli.no_git && config.create_git_repo {
        init_git_repo(target_dir)?;
    }

    if !cli.no_deps {
        add_dependencies(target_dir, config)?;
    }

    if !cli.no_verify {
        verify_build(target_dir)?;
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

    info!("Starting scaffold with project name: {}", cli.project);

    create_project(&cli, &config).context("Failed to create project")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_cli(project: &str) -> Cli {
        Cli {
            project: project.to_string(),
            author: Some("Test Author <test@example.com>".to_string()),
            directory: None,
            config: None,
            no_git: true,
            no_sample_config: false,
            no_verify: true,
            no_deps: true,
        }
    }

    fn create_test_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_create_project_validates_empty_name() {
        let cli = Cli {
            project: "".to_string(),
            author: None,
            directory: None,
            config: None,
            no_git: true,
            no_sample_config: false,
            no_verify: true,
            no_deps: true,
        };
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Project name cannot be empty"));
    }

    #[test]
    fn test_create_project_validates_name_starting_with_dash() {
        let cli = Cli {
            project: "-invalid".to_string(),
            author: None,
            directory: None,
            config: None,
            no_git: true,
            no_sample_config: false,
            no_verify: true,
            no_deps: true,
        };
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot start with '-' or '_'"));
    }

    #[test]
    fn test_create_project_validates_name_starting_with_underscore() {
        let cli = Cli {
            project: "_invalid".to_string(),
            author: None,
            directory: None,
            config: None,
            no_git: true,
            no_sample_config: false,
            no_verify: true,
            no_deps: true,
        };
        let config = create_test_config();

        let result = create_project(&cli, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot start with '-' or '_'"));
    }

    #[test]
    fn test_create_project_validates_invalid_characters() {
        let cli = Cli {
            project: "invalid@name".to_string(),
            author: None,
            directory: None,
            config: None,
            no_git: true,
            no_sample_config: false,
            no_verify: true,
            no_deps: true,
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
}
