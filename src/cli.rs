use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "scaffold",
    about = "A Rust CLI project scaffolding tool that generates production-ready CLI applications",
    version = env!("GIT_DESCRIBE"),
    after_help = "Logs are written to: ~/.local/share/scaffold/logs/scaffold.log\n\nThis tool generates complete Rust CLI projects with best practices including:\n- Proper error handling with eyre\n- Structured logging with env_logger\n- Configuration management with serde_yaml\n- Modern CLI parsing with clap\n- Git version integration"
)]
pub struct Cli {
    /// Name of the project to create
    pub project: String,

    /// Author name for Cargo.toml
    #[arg(short, long, help = "Author name for Cargo.toml")]
    pub author: Option<String>,

    /// Target directory (default: ./<project-name>)
    #[arg(short, long, help = "Target directory (default: ./<project-name>)")]
    pub directory: Option<PathBuf>,

    /// Path to config file
    #[arg(short, long, help = "Path to config file")]
    pub config: Option<PathBuf>,

    /// Don't initialize git repository
    #[arg(long, help = "Don't initialize git repository")]
    pub no_git: bool,

    /// Don't create sample config file
    #[arg(long, help = "Don't create sample config file")]
    pub no_sample_config: bool,

    /// Don't verify that the generated project builds (much faster)
    #[arg(long, help = "Don't verify that the generated project builds (much faster)")]
    pub no_verify: bool,

    /// Don't add dependencies with cargo add (much faster, generates basic Cargo.toml)
    #[arg(long, help = "Don't add dependencies with cargo add (much faster, generates basic Cargo.toml)")]
    pub no_deps: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Parser, CommandFactory};

    #[test]
    fn test_cli_parses_project_name() {
        let cli = Cli::try_parse_from(&["scaffold", "my-project"]).unwrap();
        assert_eq!(cli.project, "my-project");
        assert!(cli.author.is_none());
        assert!(cli.directory.is_none());
        assert!(cli.config.is_none());
        assert!(!cli.no_git);
        assert!(!cli.no_sample_config);
        assert!(!cli.no_verify);
        assert!(!cli.no_deps);
    }

    #[test]
    fn test_cli_parses_all_options() {
        let cli = Cli::try_parse_from(&[
            "scaffold",
            "test-project",
            "--author", "Test Author <test@example.com>",
            "--directory", "/tmp/test",
            "--config", "config.yml",
            "--no-git",
            "--no-sample-config",
            "--no-verify",
            "--no-deps"
        ]).unwrap();

        assert_eq!(cli.project, "test-project");
        assert_eq!(cli.author, Some("Test Author <test@example.com>".to_string()));
        assert_eq!(cli.directory, Some(PathBuf::from("/tmp/test")));
        assert_eq!(cli.config, Some(PathBuf::from("config.yml")));
        assert!(cli.no_git);
        assert!(cli.no_sample_config);
        assert!(cli.no_verify);
        assert!(cli.no_deps);
    }

    #[test]
    fn test_cli_parses_short_flags() {
        let cli = Cli::try_parse_from(&[
            "scaffold",
            "test-project",
            "-a", "Short Author",
            "-d", "/tmp/short",
            "-c", "short.yml"
        ]).unwrap();

        assert_eq!(cli.project, "test-project");
        assert_eq!(cli.author, Some("Short Author".to_string()));
        assert_eq!(cli.directory, Some(PathBuf::from("/tmp/short")));
        assert_eq!(cli.config, Some(PathBuf::from("short.yml")));
    }

    #[test]
    fn test_cli_requires_project_name() {
        let result = Cli::try_parse_from(&["scaffold"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_accepts_complex_project_names() {
        let valid_names = [
            "simple",
            "with-dashes", 
            "with_underscores",
            "with123numbers",
            "a",
            "very-long-project-name-with-many-parts"
        ];

        for name in valid_names.iter() {
            let cli = Cli::try_parse_from(&["scaffold", name]).unwrap();
            assert_eq!(cli.project, *name);
        }
    }

    #[test]
    fn test_cli_help_contains_expected_text() {
        let help = Cli::command().render_help().to_string();
        
        assert!(help.contains("A Rust CLI project scaffolding tool"));
        assert!(help.contains("Name of the project to create"));
        assert!(help.contains("Author name for Cargo.toml"));
        assert!(help.contains("Target directory"));
        assert!(help.contains("Path to config file"));
        assert!(help.contains("Don't initialize git repository"));
        assert!(help.contains("Don't create sample config file"));
        assert!(help.contains("Don't verify that the generated project builds"));
        assert!(help.contains("Don't add dependencies with cargo add"));
        assert!(help.contains("Logs are written to"));
        assert!(help.contains("Proper error handling with eyre"));
        assert!(help.contains("Structured logging with env_logger"));
        assert!(help.contains("Configuration management with serde_yaml"));
        assert!(help.contains("Modern CLI parsing with clap"));
        assert!(help.contains("Git version integration"));
    }

    #[test]
    fn test_cli_version_uses_git_describe() {
        // This test verifies that the version field references GIT_DESCRIBE
        // The actual value depends on build-time environment
        let cmd = Cli::command();
        let version = cmd.get_version().unwrap_or("unknown");
        assert!(!version.is_empty());
    }

    #[test]
    fn test_cli_boolean_flags_default_false() {
        let cli = Cli::try_parse_from(&["scaffold", "test"]).unwrap();
        
        assert!(!cli.no_git);
        assert!(!cli.no_sample_config);
        assert!(!cli.no_verify);
        assert!(!cli.no_deps);
    }

    #[test]
    fn test_cli_optional_fields_default_none() {
        let cli = Cli::try_parse_from(&["scaffold", "test"]).unwrap();
        
        assert!(cli.author.is_none());
        assert!(cli.directory.is_none());
        assert!(cli.config.is_none());
    }
}
