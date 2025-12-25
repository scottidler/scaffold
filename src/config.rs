use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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
        sample_config.insert(
            "age".to_string(),
            serde_yaml::Value::Number(serde_yaml::Number::from(30)),
        );
        sample_config.insert("debug".to_string(), serde_yaml::Value::Bool(false));

        Self {
            create_build_rs: true,
            create_cli_module: true,
            create_config_module: true,
            dependencies: vec![
                Dependency {
                    name: "clap".to_string(),
                    features: vec!["derive".to_string()],
                },
                Dependency {
                    name: "eyre".to_string(),
                    features: vec![],
                },
                Dependency {
                    name: "log".to_string(),
                    features: vec![],
                },
                Dependency {
                    name: "env_logger".to_string(),
                    features: vec![],
                },
                Dependency {
                    name: "serde".to_string(),
                    features: vec!["derive".to_string()],
                },
                Dependency {
                    name: "serde_yaml".to_string(),
                    features: vec![],
                },
                Dependency {
                    name: "dirs".to_string(),
                    features: vec![],
                },
                Dependency {
                    name: "colored".to_string(),
                    features: vec![],
                },
            ],
            sample_config,
            cli: CliConfig::default(),
        }
    }
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            after_help: "Logs are written to: ~/.local/share/{{PROJECT}}/logs/{{PROJECT}}.log".to_string(),
        }
    }
}

impl Config {
    /// Load configuration with fallback chain
    pub fn load(config_path: Option<&PathBuf>) -> Result<Self> {
        // If explicit config path provided, try to load it
        if let Some(path) = config_path {
            return Self::load_from_file(path).context(format!("Failed to load config from {}", path.display()));
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
        let content = fs::read_to_string(&path).context("Failed to read config file")?;

        let config: Self = serde_yaml::from_str(&content).context("Failed to parse config file")?;

        log::info!("Loaded config from: {}", path.as_ref().display());
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_default_values() {
        let config = Config::default();

        assert_eq!(config.default_author, "Your Name <your.email@example.com>");
        assert_eq!(config.default_license, "MIT");
        assert!(config.create_git_repo);
        assert!(config.create_sample_config);
        assert!(!config.debug);
    }

    #[test]
    fn test_template_config_default_values() {
        let template = TemplateConfig::default();

        assert!(template.create_build_rs);
        assert!(template.create_cli_module);
        assert!(template.create_config_module);
        assert!(!template.dependencies.is_empty());
        assert!(!template.sample_config.is_empty());
    }

    #[test]
    fn test_template_config_default_dependencies() {
        let template = TemplateConfig::default();

        let dep_names: Vec<&str> = template.dependencies.iter().map(|d| d.name.as_str()).collect();
        assert!(dep_names.contains(&"clap"));
        assert!(dep_names.contains(&"eyre"));
        assert!(dep_names.contains(&"log"));
        assert!(dep_names.contains(&"env_logger"));
        assert!(dep_names.contains(&"serde"));
        assert!(dep_names.contains(&"serde_yaml"));
        assert!(dep_names.contains(&"dirs"));
        assert!(dep_names.contains(&"colored"));
    }

    #[test]
    fn test_template_config_clap_has_derive_feature() {
        let template = TemplateConfig::default();

        let clap_dep = template.dependencies.iter().find(|d| d.name == "clap").unwrap();
        assert!(clap_dep.features.contains(&"derive".to_string()));
    }

    #[test]
    fn test_template_config_serde_has_derive_feature() {
        let template = TemplateConfig::default();

        let serde_dep = template.dependencies.iter().find(|d| d.name == "serde").unwrap();
        assert!(serde_dep.features.contains(&"derive".to_string()));
    }

    #[test]
    fn test_template_config_default_sample_config() {
        let template = TemplateConfig::default();

        assert!(template.sample_config.contains_key("name"));
        assert!(template.sample_config.contains_key("age"));
        assert!(template.sample_config.contains_key("debug"));

        if let Some(serde_yaml::Value::String(name)) = template.sample_config.get("name") {
            assert_eq!(name, "John Doe");
        } else {
            panic!("Expected name to be a string");
        }

        if let Some(serde_yaml::Value::Number(age)) = template.sample_config.get("age") {
            assert_eq!(age.as_u64(), Some(30));
        } else {
            panic!("Expected age to be a number");
        }

        if let Some(serde_yaml::Value::Bool(debug)) = template.sample_config.get("debug") {
            assert!(!debug);
        } else {
            panic!("Expected debug to be a boolean");
        }
    }

    #[test]
    fn test_cli_config_default_values() {
        let cli_config = CliConfig::default();

        assert!(cli_config.after_help.contains("{{PROJECT}}"));
        assert!(cli_config.after_help.contains("logs"));
    }

    #[test]
    fn test_config_load_with_no_file_returns_default() {
        // Test that default config has expected values
        let config = Config::default();

        assert_eq!(config.default_author, "Your Name <your.email@example.com>");
        assert_eq!(config.default_license, "MIT");
        assert!(config.create_git_repo);
    }

    #[test]
    fn test_config_load_from_explicit_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.yml");

        let config_content = r#"
default_author: "Test Author <test@example.com>"
default_license: "Apache-2.0"
create_git_repo: false
create_sample_config: false
debug: true
template:
  create_build_rs: false
  dependencies:
    - name: "custom-dep"
      features: ["feature1", "feature2"]
  sample_config:
    custom_field: "custom_value"
  cli:
    after_help: "Custom help text"
"#;

        fs::write(&config_file, config_content).unwrap();

        let config = Config::load(Some(&config_file)).unwrap();

        assert_eq!(config.default_author, "Test Author <test@example.com>");
        assert_eq!(config.default_license, "Apache-2.0");
        assert!(!config.create_git_repo);
        assert!(!config.create_sample_config);
        assert!(config.debug);
        assert!(!config.template.create_build_rs);
        assert_eq!(config.template.cli.after_help, "Custom help text");

        // Check custom dependency
        let custom_dep = config
            .template
            .dependencies
            .iter()
            .find(|d| d.name == "custom-dep")
            .unwrap();
        assert!(custom_dep.features.contains(&"feature1".to_string()));
        assert!(custom_dep.features.contains(&"feature2".to_string()));

        // Check custom sample config
        assert!(config.template.sample_config.contains_key("custom_field"));
    }

    #[test]
    fn test_config_load_from_nonexistent_file_returns_error() {
        let nonexistent_file = PathBuf::from("/this/file/does/not/exist.yml");

        let result = Config::load(Some(&nonexistent_file));
        assert!(result.is_err());
    }

    #[test]
    fn test_config_load_from_invalid_yaml_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("invalid.yml");

        let invalid_content = "invalid: yaml: content: [";
        fs::write(&config_file, invalid_content).unwrap();

        let result = Config::load(Some(&config_file));
        assert!(result.is_err());
    }

    #[test]
    fn test_dependency_serialization() {
        let dep = Dependency {
            name: "test-dep".to_string(),
            features: vec!["feature1".to_string(), "feature2".to_string()],
        };

        let yaml = serde_yaml::to_string(&dep).unwrap();
        assert!(yaml.contains("name: test-dep"));
        assert!(yaml.contains("feature1"));
        assert!(yaml.contains("feature2"));

        let deserialized: Dependency = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(deserialized.name, "test-dep");
        assert_eq!(deserialized.features.len(), 2);
    }

    #[test]
    fn test_dependency_default_features() {
        let yaml = "name: test-dep";
        let dep: Dependency = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(dep.name, "test-dep");
        assert!(dep.features.is_empty());
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let original_config = Config::default();

        let yaml = serde_yaml::to_string(&original_config).unwrap();
        let deserialized_config: Config = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(original_config.default_author, deserialized_config.default_author);
        assert_eq!(original_config.default_license, deserialized_config.default_license);
        assert_eq!(original_config.create_git_repo, deserialized_config.create_git_repo);
        assert_eq!(
            original_config.create_sample_config,
            deserialized_config.create_sample_config
        );
        assert_eq!(original_config.debug, deserialized_config.debug);
        assert_eq!(
            original_config.template.dependencies.len(),
            deserialized_config.template.dependencies.len()
        );
    }
}
