use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub default_author: String,
    pub default_license: String,
    pub create_git_repo: bool,
    pub create_sample_config: bool,
    pub debug: bool,
    pub template: TemplateConfig,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct TemplateConfig {
    pub create_build_rs: bool,
    pub create_cli_module: bool,
    pub create_config_module: bool,
    pub dependencies: Vec<Dependency>,
    pub sample_config: HashMap<String, serde_yaml::Value>,
    pub cli: CliConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Dependency {
    pub name: String,
    #[serde(default)]
    pub features: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct CliConfig {
    pub after_help: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_author: "Your Name <your.email@example.com>".to_string(),
            default_license: "MIT".to_string(),
            create_git_repo: true,
            create_sample_config: true,
            debug: false,
            template: TemplateConfig::default(),
        }
    }
}

impl Default for TemplateConfig {
    fn default() -> Self {
        let mut sample_config = HashMap::new();
        sample_config.insert("name".to_string(), serde_yaml::Value::String("John Doe".to_string()));
        sample_config.insert("age".to_string(), serde_yaml::Value::Number(serde_yaml::Number::from(30)));
        sample_config.insert("debug".to_string(), serde_yaml::Value::Bool(false));
        
        Self {
            create_build_rs: true,
            create_cli_module: true,
            create_config_module: true,
            dependencies: vec![
                Dependency { name: "clap".to_string(), features: vec!["derive".to_string()] },
                Dependency { name: "eyre".to_string(), features: vec![] },
                Dependency { name: "log".to_string(), features: vec![] },
                Dependency { name: "env_logger".to_string(), features: vec![] },
                Dependency { name: "serde".to_string(), features: vec!["derive".to_string()] },
                Dependency { name: "serde_yaml".to_string(), features: vec![] },
                Dependency { name: "dirs".to_string(), features: vec![] },
                Dependency { name: "colored".to_string(), features: vec![] },
            ],
            sample_config,
            cli: CliConfig::default(),
        }
    }
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            after_help: "Logs are written to: ~/.local/share/{{PROJECT_NAME}}/logs/{{PROJECT_NAME}}.log".to_string(),
        }
    }
}

impl Config {
    /// Load configuration with fallback chain
    pub fn load(config_path: Option<&PathBuf>) -> Result<Self> {
        // If explicit config path provided, try to load it
        if let Some(path) = config_path {
            return Self::load_from_file(path)
                .context(format!("Failed to load config from {}", path.display()));
        }
        
        // Try primary location: ~/.config/scaffold/scaffold.yml
        if let Some(config_dir) = dirs::config_dir() {
            let primary_config = config_dir.join("scaffold").join("scaffold.yml");
            if primary_config.exists() {
                match Self::load_from_file(&primary_config) {
                    Ok(config) => return Ok(config),
                    Err(e) => {
                        log::warn!("Failed to load config from {}: {}", primary_config.display(), e);
                    }
                }
            }
        }
        
        // Try fallback location: ./scaffold.yml
        let fallback_config = PathBuf::from("scaffold.yml");
        if fallback_config.exists() {
            match Self::load_from_file(&fallback_config) {
                Ok(config) => return Ok(config),
                Err(e) => {
                    log::warn!("Failed to load config from {}: {}", fallback_config.display(), e);
                }
            }
        }
        
        // No config file found, use defaults
        log::info!("No config file found, using defaults");
        Ok(Self::default())
    }
    
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .context("Failed to read config file")?;
        
        let config: Self = serde_yaml::from_str(&content)
            .context("Failed to parse config file")?;
        
        log::info!("Loaded config from: {}", path.as_ref().display());
        Ok(config)
    }
    

} 