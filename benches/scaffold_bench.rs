use criterion::{criterion_group, criterion_main, Criterion};
use std::process::Command;
use tempfile::TempDir;

fn benchmark_project_creation(c: &mut Criterion) {
    c.bench_function("create_basic_project", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let project_path = temp_dir.path().join("bench-project");
            
            let output = Command::new("cargo")
                .args(&["run", "--", "bench-project", "--directory", &project_path.to_string_lossy()])
                .output()
                .expect("Failed to run scaffold");
            
            assert!(output.status.success(), "Scaffold command failed");
        })
    });
}

fn benchmark_config_loading(c: &mut Criterion) {
    use scaffold::config::Config;
    
    c.bench_function("load_default_config", |b| {
        b.iter(|| {
            let _config = Config::load(None).expect("Failed to load config");
        })
    });
}

fn benchmark_template_generation(c: &mut Criterion) {
    use scaffold::templates;
    use std::path::PathBuf;
    
    c.bench_function("generate_all_templates", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let project_path = temp_dir.path().to_path_buf();
            
            templates::generate_project("bench-test", &project_path, "Test Author")
                .expect("Failed to generate templates");
        })
    });
}

criterion_group!(benches, benchmark_project_creation, benchmark_config_loading, benchmark_template_generation);
criterion_main!(benches); 