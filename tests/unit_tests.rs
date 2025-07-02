use scaffold::config::{Config, TemplateConfig, Dependency, CliConfig};
use std::collections::HashMap;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_config_default_values() {
    let config = Config::default();
    
    assert_eq!(config.default_author, "Your Name <your.email@example.com>");
    assert_eq!(config.default_license, "MIT");
    assert!(config.create_git_repo);
    assert!(config.create_sample_config);
    assert!(!config.debug);
    
    // Test template defaults
    assert!(config.template.create_build_rs);
    assert!(config.template.create_cli_module);
    assert!(config.template.create_config_module);
    assert!(!config.template.dependencies.is_empty());
    assert!(!config.template.sample_config.is_empty());
}

#[test]
fn test_template_config_default_dependencies() {
    let template = TemplateConfig::default();
    
    let expected_deps = ["clap", "eyre", "log", "env_logger", "serde", "serde_yaml", "dirs", "colored"];
    
    for expected_dep in expected_deps.iter() {
        assert!(template.dependencies.iter().any(|dep| dep.name == *expected_dep),
                "Missing expected dependency: {}", expected_dep);
    }
    
    // Check specific feature requirements
    let clap_dep = template.dependencies.iter().find(|dep| dep.name == "clap").unwrap();
    assert!(clap_dep.features.contains(&"derive".to_string()));
    
    let serde_dep = template.dependencies.iter().find(|dep| dep.name == "serde").unwrap();
    assert!(serde_dep.features.contains(&"derive".to_string()));
}

#[test]
fn test_template_config_default_sample_config() {
    let template = TemplateConfig::default();
    
    assert!(template.sample_config.contains_key("name"));
    assert!(template.sample_config.contains_key("age"));
    assert!(template.sample_config.contains_key("debug"));
    
    // Verify types
    match template.sample_config.get("name").unwrap() {
        serde_yaml::Value::String(s) => assert_eq!(s, "John Doe"),
        _ => panic!("Expected string value for name"),
    }
    
    match template.sample_config.get("age").unwrap() {
        serde_yaml::Value::Number(n) => assert_eq!(n.as_i64().unwrap(), 30),
        _ => panic!("Expected number value for age"),
    }
    
    match template.sample_config.get("debug").unwrap() {
        serde_yaml::Value::Bool(b) => assert!(!b),
        _ => panic!("Expected boolean value for debug"),
    }
}

#[test]
fn test_cli_config_default() {
    let cli_config = CliConfig::default();
    assert!(cli_config.after_help.contains("{{PROJECT_NAME}}"));
    assert!(cli_config.after_help.contains("~/.local/share"));
    assert!(cli_config.after_help.contains("logs"));
}

#[test]
fn test_dependency_serialization() {
    let dep = Dependency {
        name: "test-crate".to_string(),
        features: vec!["feature1".to_string(), "feature2".to_string()],
    };
    
    let serialized = serde_yaml::to_string(&dep).unwrap();
    assert!(serialized.contains("name: test-crate"));
    assert!(serialized.contains("feature1"));
    assert!(serialized.contains("feature2"));
    
    let deserialized: Dependency = serde_yaml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.name, dep.name);
    assert_eq!(deserialized.features, dep.features);
}

#[test]
fn test_config_load_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.yml");
    
    let config_content = r#"
default_author: "Test Author <test@example.com>"
default_license: "Apache-2.0"
create_git_repo: false
create_sample_config: true
debug: true
template:
  create_build_rs: true
  create_cli_module: true
  create_config_module: true
  dependencies:
    - name: "custom-crate"
      features: ["custom-feature"]
  sample_config:
    custom_field: "custom_value"
  cli:
    after_help: "Custom help text"
"#;
    
    fs::write(&config_path, config_content).unwrap();
    
    let config = Config::load(Some(&config_path)).unwrap();
    
    assert_eq!(config.default_author, "Test Author <test@example.com>");
    assert_eq!(config.default_license, "Apache-2.0");
    assert!(!config.create_git_repo);
    assert!(config.debug);
    
    assert_eq!(config.template.dependencies.len(), 1);
    assert_eq!(config.template.dependencies[0].name, "custom-crate");
    assert_eq!(config.template.dependencies[0].features, vec!["custom-feature"]);
    
    assert_eq!(config.template.cli.after_help, "Custom help text");
}

#[test]
fn test_config_load_fallback_to_defaults() {
    let temp_dir = TempDir::new().unwrap();
    let non_existent_path = temp_dir.path().join("non-existent.yml");
    
    // Change to temp directory to avoid picking up scaffold.yml
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    
    // Should fall back to defaults when file doesn't exist
    let config = Config::load(None).unwrap();
    assert_eq!(config.default_author, "Your Name <your.email@example.com>");
    
    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
    
    // Should fail when explicit path doesn't exist
    let result = Config::load(Some(&non_existent_path));
    assert!(result.is_err());
}

#[test]
fn test_config_partial_loading() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("partial-config.yml");
    
    // Only specify some fields - others should use defaults
    let config_content = r#"
default_author: "Partial Author"
debug: true
"#;
    
    fs::write(&config_path, config_content).unwrap();
    
    let config = Config::load(Some(&config_path)).unwrap();
    
    assert_eq!(config.default_author, "Partial Author");
    assert!(config.debug);
    // These should be defaults
    assert_eq!(config.default_license, "MIT");
    assert!(config.create_git_repo);
    assert!(!config.template.dependencies.is_empty());
}

#[test]
fn test_dependency_features_optional() {
    let dep_with_features = Dependency {
        name: "with-features".to_string(),
        features: vec!["feature1".to_string()],
    };
    
    let dep_without_features = Dependency {
        name: "without-features".to_string(),
        features: vec![],
    };
    
    let serialized_with = serde_yaml::to_string(&dep_with_features).unwrap();
    let serialized_without = serde_yaml::to_string(&dep_without_features).unwrap();
    
    assert!(serialized_with.contains("features"));
    // Empty features should still serialize
    assert!(serialized_without.contains("features: []"));
    
    // Test deserialization
    let yaml_without_features = r#"
name: "test-crate"
"#;
    
    let deserialized: Dependency = serde_yaml::from_str(yaml_without_features).unwrap();
    assert_eq!(deserialized.name, "test-crate");
    assert!(deserialized.features.is_empty());
}

#[test]
fn test_config_yaml_roundtrip() {
    let original_config = Config::default();
    
    let serialized = serde_yaml::to_string(&original_config).unwrap();
    let deserialized: Config = serde_yaml::from_str(&serialized).unwrap();
    
    assert_eq!(original_config.default_author, deserialized.default_author);
    assert_eq!(original_config.default_license, deserialized.default_license);
    assert_eq!(original_config.create_git_repo, deserialized.create_git_repo);
    assert_eq!(original_config.debug, deserialized.debug);
    
    assert_eq!(original_config.template.dependencies.len(), 
               deserialized.template.dependencies.len());
    
    for (orig, deser) in original_config.template.dependencies.iter()
                           .zip(deserialized.template.dependencies.iter()) {
        assert_eq!(orig.name, deser.name);
        assert_eq!(orig.features, deser.features);
    }
}

#[test]
fn test_template_config_sample_config_types() {
    let mut sample_config = HashMap::new();
    sample_config.insert("string_field".to_string(), 
                        serde_yaml::Value::String("test".to_string()));
    sample_config.insert("number_field".to_string(), 
                        serde_yaml::Value::Number(serde_yaml::Number::from(42)));
    sample_config.insert("bool_field".to_string(), 
                        serde_yaml::Value::Bool(true));
    
    let template = TemplateConfig {
        create_build_rs: true,
        create_cli_module: true,
        create_config_module: true,
        dependencies: vec![],
        sample_config,
        cli: CliConfig::default(),
    };
    
    let serialized = serde_yaml::to_string(&template).unwrap();
    let deserialized: TemplateConfig = serde_yaml::from_str(&serialized).unwrap();
    
    assert_eq!(deserialized.sample_config.len(), 3);
    
    match deserialized.sample_config.get("string_field").unwrap() {
        serde_yaml::Value::String(s) => assert_eq!(s, "test"),
        _ => panic!("Expected string value"),
    }
    
    match deserialized.sample_config.get("number_field").unwrap() {
        serde_yaml::Value::Number(n) => assert_eq!(n.as_i64().unwrap(), 42),
        _ => panic!("Expected number value"),
    }
    
    match deserialized.sample_config.get("bool_field").unwrap() {
        serde_yaml::Value::Bool(b) => assert!(b),
        _ => panic!("Expected boolean value"),
    }
} 