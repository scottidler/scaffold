use clap::Parser;
use colored::*;
use eyre::{Context, Result};
use log::{info, warn, error};

use std::fs;
use std::path::PathBuf;
use std::process::Command;

mod cli;
mod config;
mod templates;

use cli::Cli;
use config::Config;

fn setup_logging() -> Result<()> {
    // Create log directory
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("scaffold")
        .join("logs");
    
    fs::create_dir_all(&log_dir)
        .context("Failed to create log directory")?;
    
    let log_file = log_dir.join("scaffold.log");
    
    // Setup env_logger with file output
    let target = Box::new(fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .context("Failed to open log file")?);
    
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(target))
        .init();
    
    info!("Logging initialized, writing to: {}", log_file.display());
    Ok(())
}

fn create_project(cli: &Cli, config: &Config) -> Result<()> {
    let project_name = &cli.project_name;
    let default_dir = PathBuf::from(project_name);
    let target_dir = cli.directory.as_ref()
        .unwrap_or(&default_dir);
    
    // Validate project name
    if !project_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(eyre::eyre!("Project name must contain only alphanumeric characters, hyphens, and underscores"));
    }
    
    info!("Creating project: {}", project_name);
    println!("{} Creating project: {}", "✓".green(), project_name.cyan());
    
    // Check if directory exists
    if target_dir.exists() {
        if target_dir.read_dir()?.next().is_some() {
            return Err(eyre::eyre!("Directory {} already exists and is not empty", target_dir.display()));
        }
    }
    
    // Create project directory
    fs::create_dir_all(target_dir)
        .context("Failed to create project directory")?;
    
    println!("{} Created directory: {}", "✓".green(), target_dir.display());
    
    // Generate all project files
    templates::generate_project(project_name, target_dir, &cli.author.as_ref().unwrap_or(&config.default_author))?;
    
    // Initialize git repository
    if !cli.no_git && config.create_git_repo {
        init_git_repo(target_dir)?;
    }
    
    // Add dependencies
    add_dependencies(target_dir, config)?;
    
    // Verify the build works
    verify_build(target_dir)?;
    
    println!("\n{} Project {} created successfully!", "🎉".green(), project_name.cyan());
    println!("Next steps:");
    println!("  cd {}", target_dir.display());
    println!("  cargo run");
    
    Ok(())
}

fn init_git_repo(target_dir: &PathBuf) -> Result<()> {
    info!("Initializing git repository");
    let output = Command::new("git")
        .args(&["init"])
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
        cmd.args(&["add", &dep.name])
            .current_dir(target_dir);
        
        if !dep.features.is_empty() {
            let features = format!("--features={}", dep.features.join(","));
            cmd.arg(features);
        }
        
        let output = cmd.output()
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
        .args(&["build"])
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
    // Setup logging first
    setup_logging()
        .context("Failed to setup logging")?;
    
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Load configuration
    let config = Config::load(cli.config.as_ref())
        .context("Failed to load configuration")?;
    
    info!("Starting scaffold with project name: {}", cli.project_name);
    
    // Create the project
    create_project(&cli, &config)
        .context("Failed to create project")?;
    
    Ok(())
} 