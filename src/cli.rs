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
}
