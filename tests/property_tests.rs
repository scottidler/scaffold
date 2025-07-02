use proptest::prelude::*;
use assert_cmd::Command;
use tempfile::TempDir;
use std::fs;

// Helper function to generate valid project names
fn valid_project_name() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"[a-zA-Z][a-zA-Z0-9_-]{0,49}")
        .unwrap()
        .prop_filter("Project name should not be empty and not start with dash", |s| {
            !s.is_empty() && !s.starts_with('-') && !s.starts_with('_')
        })
}

// Helper function to generate valid author strings
fn valid_author() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"[a-zA-Z0-9 ._-]{1,100} <[a-zA-Z0-9._-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}>")
        .unwrap()
}

// Helper function to scaffold command
fn scaffold_cmd() -> Command {
    Command::cargo_bin("scaffold").expect("Failed to find scaffold binary")
}

proptest! {
    #[test]
    fn test_valid_project_names_succeed(project_name in valid_project_name()) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join(&project_name);
        
        scaffold_cmd()
            .arg(&project_name)
            .arg("--directory")
            .arg(&project_path)
            .arg("--no-git") // Faster without git
            .assert()
            .success();
        
        // Verify basic structure exists
        prop_assert!(project_path.exists());
        prop_assert!(project_path.join("Cargo.toml").exists());
        prop_assert!(project_path.join("src/main.rs").exists());
    }
    
    #[test]
    fn test_valid_authors_in_cargo_toml(
        project_name in valid_project_name(),
        author in valid_author()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join(&project_name);
        
        scaffold_cmd()
            .arg(&project_name)
            .arg("--directory")
            .arg(&project_path)
            .arg("--author")
            .arg(&author)
            .arg("--no-git")
            .assert()
            .success();
        
        let cargo_toml = fs::read_to_string(project_path.join("Cargo.toml")).unwrap();
        prop_assert!(cargo_toml.contains(&author));
    }
    
    #[test]
    fn test_invalid_project_names_fail(
        invalid_name in prop::string::string_regex(r"[^a-zA-Z0-9_-]+").unwrap()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("temp");
        
        scaffold_cmd()
            .arg(&invalid_name)
            .arg("--directory")
            .arg(&project_path)
            .assert()
            .failure();
    }
    
    #[test]
    fn test_generated_cargo_toml_structure(project_name in valid_project_name()) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join(&project_name);
        
        scaffold_cmd()
            .arg(&project_name)
            .arg("--directory")
            .arg(&project_path)
            .arg("--no-git")
            .assert()
            .success();
        
        let cargo_toml = fs::read_to_string(project_path.join("Cargo.toml")).unwrap();
        
        // Verify required TOML structure
        let name_check = format!("name = \"{}\"", project_name);
        prop_assert!(cargo_toml.contains(&name_check));
        prop_assert!(cargo_toml.contains("version = \"0.1.0\""));
        prop_assert!(cargo_toml.contains("edition = \"2024\""));
        prop_assert!(cargo_toml.contains("[dependencies]"));
        prop_assert!(cargo_toml.contains("build = \"build.rs\""));
    }
    
    #[test]
    fn test_generated_main_rs_contains_project_name(project_name in valid_project_name()) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join(&project_name);
        
        scaffold_cmd()
            .arg(&project_name)
            .arg("--directory")
            .arg(&project_path)
            .arg("--no-git")
            .assert()
            .success();
        
        let main_rs = fs::read_to_string(project_path.join("src/main.rs")).unwrap();
        
        // Project name should appear in logging setup
        prop_assert!(main_rs.contains(&project_name));
        
        // Verify essential imports and structure
        prop_assert!(main_rs.contains("use clap::Parser"));
        prop_assert!(main_rs.contains("use eyre::"));
        prop_assert!(main_rs.contains("Context"));
        prop_assert!(main_rs.contains("Result"));
        prop_assert!(main_rs.contains("fn main() -> Result<()>"));
        prop_assert!(main_rs.contains("fn setup_logging()"));
    }
    
    #[test]
    fn test_generated_cli_rs_structure(project_name in valid_project_name()) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join(&project_name);
        
        scaffold_cmd()
            .arg(&project_name)
            .arg("--directory")
            .arg(&project_path)
            .arg("--no-git")
            .assert()
            .success();
        
        let cli_rs = fs::read_to_string(project_path.join("src/cli.rs")).unwrap();
        
        // Verify CLI structure with project name
        let cli_name_check = format!("name = \"{}\"", project_name);
        prop_assert!(cli_rs.contains(&cli_name_check));
        prop_assert!(cli_rs.contains("#[derive(Parser)]"));
        prop_assert!(cli_rs.contains("pub struct Cli"));
        prop_assert!(cli_rs.contains("config: Option<PathBuf>"));
        prop_assert!(cli_rs.contains("verbose: bool"));
        
        // Verify log path contains project name
        let log_path_check = format!("~/.local/share/{}/logs/{}.log", project_name, project_name);
        prop_assert!(cli_rs.contains(&log_path_check));
    }
    
    #[test]
    fn test_sample_config_file_created(project_name in valid_project_name()) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join(&project_name);
        
        scaffold_cmd()
            .arg(&project_name)
            .arg("--directory")
            .arg(&project_path)
            .arg("--no-git")
            .assert()
            .success();
        
        let config_file = project_path.join(format!("{}.yml", project_name));
        prop_assert!(config_file.exists());
        
        let config_content = fs::read_to_string(&config_file).unwrap();
        prop_assert!(config_content.contains("name: \"John Doe\""));
        prop_assert!(config_content.contains("age: 30"));
        prop_assert!(config_content.contains("debug: false"));
    }
}

// Additional targeted property tests for edge cases
proptest! {
    #[test]
    fn test_project_name_length_limits(
        short_name in prop::string::string_regex(r"[a-zA-Z]{1}").unwrap(),
        long_name in prop::string::string_regex(r"[a-zA-Z0-9_-]{50}").unwrap()
    ) {
        let temp_dir = TempDir::new().unwrap();
        
        // Test very short name
        let short_path = temp_dir.path().join(&short_name);
        scaffold_cmd()
            .arg(&short_name)
            .arg("--directory")
            .arg(&short_path)
            .arg("--no-git")
            .assert()
            .success();
        
        // Test long name
        let long_path = temp_dir.path().join(&long_name);
        scaffold_cmd()
            .arg(&long_name)
            .arg("--directory")
            .arg(&long_path)
            .arg("--no-git")
            .assert()
            .success();
    }
    
    #[test]
    fn test_special_characters_in_project_names(
        name_with_hyphens in prop::string::string_regex(r"[a-zA-Z0-9]+-[a-zA-Z0-9]+-[a-zA-Z0-9]+").unwrap(),
        name_with_underscores in prop::string::string_regex(r"[a-zA-Z0-9]+_[a-zA-Z0-9]+_[a-zA-Z0-9]+").unwrap()
    ) {
        let temp_dir = TempDir::new().unwrap();
        
        // Test hyphens
        let hyphen_path = temp_dir.path().join(&name_with_hyphens);
        scaffold_cmd()
            .arg(&name_with_hyphens)
            .arg("--directory")
            .arg(&hyphen_path)
            .arg("--no-git")
            .assert()
            .success();
        
        // Test underscores
        let underscore_path = temp_dir.path().join(&name_with_underscores);
        scaffold_cmd()
            .arg(&name_with_underscores)
            .arg("--directory")
            .arg(&underscore_path)
            .arg("--no-git")
            .assert()
            .success();
    }
}

// Test configuration property preservation
proptest! {
    #[test]
    fn test_config_roundtrip_properties(
        author in valid_author(),
        debug in any::<bool>(),
        create_git in any::<bool>()
    ) {
        use scaffold::config::Config;
        use tempfile::NamedTempFile;
        
        // Create a config with random values
        let mut config = Config::default();
        config.default_author = author.clone();
        config.debug = debug;
        config.create_git_repo = create_git;
        
        // Serialize to YAML
        let yaml = serde_yaml::to_string(&config).unwrap();
        
        // Write to temp file
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), &yaml).unwrap();
        
        // Load back from file
        let loaded_config = Config::load(Some(&temp_file.path().to_path_buf())).unwrap();
        
        // Verify properties preserved
        prop_assert_eq!(loaded_config.default_author, author);
        prop_assert_eq!(loaded_config.debug, debug);
        prop_assert_eq!(loaded_config.create_git_repo, create_git);
    }
} 