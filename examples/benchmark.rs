use std::collections::HashMap;
use std::fs;
use std::hint::black_box;
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::TempDir;

use repo_lint::config::{CaseStyle, ConfigIR, LayoutNode, Mode, RulesConfig};
use repo_lint::engine::{FileMatcher, Walker};

fn create_test_layout() -> LayoutNode {
    let mut module_children = HashMap::new();
    module_children.insert("index.ts".to_string(), LayoutNode::file());

    let mut api_children = HashMap::new();
    api_children.insert("index.ts".to_string(), LayoutNode::file());

    let mut domain_children = HashMap::new();
    domain_children.insert("entities".to_string(), LayoutNode::dir(HashMap::new()));

    let mut service_children = HashMap::new();
    service_children.insert("api".to_string(), LayoutNode::dir(api_children));
    service_children.insert("domain".to_string(), LayoutNode::dir(domain_children));

    let mut services_children = HashMap::new();
    services_children.insert(
        "$module".to_string(),
        LayoutNode::param("module", CaseStyle::Kebab, LayoutNode::dir(service_children)),
    );

    let mut src_children = HashMap::new();
    src_children.insert("services".to_string(), LayoutNode::dir(services_children));

    let mut root_children = HashMap::new();
    root_children.insert("src".to_string(), LayoutNode::dir(src_children));

    LayoutNode::dir(root_children)
}

fn create_test_config() -> ConfigIR {
    ConfigIR {
        mode: Mode::Strict,
        layout: create_test_layout(),
        rules: RulesConfig {
            forbid_paths: vec!["**/utils/**".to_string(), "**/*.bak".to_string()],
            forbid_names: vec!["temp".to_string(), "new".to_string()],
        },
        boundaries: None,
        deps: None,
    }
}

fn create_large_directory_structure(root: &Path, num_modules: usize, files_per_module: usize) {
    fs::create_dir_all(root.join("src/services")).unwrap();

    for i in 0..num_modules {
        let module_name = format!("module-{:04}", i);
        let module_path = root.join(format!("src/services/{}", module_name));

        fs::create_dir_all(module_path.join("api")).unwrap();
        fs::create_dir_all(module_path.join("domain/entities")).unwrap();

        fs::write(module_path.join("api/index.ts"), "export {};").unwrap();

        for j in 0..files_per_module {
            fs::write(
                module_path.join(format!("domain/entities/entity-{}.ts", j)),
                "export {};",
            )
            .unwrap();
        }
    }
}

fn bench_walker_throughput() {
    println!("\n=== Walker Throughput Benchmark ===\n");

    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let num_modules = 100;
    let files_per_module = 50;
    let expected_files = num_modules * (2 + files_per_module);

    println!(
        "Creating {} modules with {} files each ({} total files)...",
        num_modules, files_per_module, expected_files
    );

    create_large_directory_structure(root, num_modules, files_per_module);

    let walker = Walker::new(root);

    let iterations = 5;
    let mut durations = Vec::new();

    for i in 0..iterations {
        let start = Instant::now();
        let entries = walker.walk();
        let duration = start.elapsed();

        durations.push(duration);
        println!(
            "  Run {}: {} files in {:?}",
            i + 1,
            entries.len(),
            duration
        );
    }

    let avg_duration: Duration = durations.iter().sum::<Duration>() / iterations as u32;
    let paths_per_sec = expected_files as f64 / avg_duration.as_secs_f64();

    println!("\n  Average: {:?}", avg_duration);
    println!("  Throughput: {:.0} paths/sec", paths_per_sec);

    if paths_per_sec >= 200_000.0 {
        println!("  [PASS] Meets target of 200k paths/sec");
    } else {
        println!(
            "  [INFO] Below 200k target (expected with small test set and SSD overhead)"
        );
    }
}

fn bench_matcher_throughput() {
    println!("\n=== Matcher Throughput Benchmark ===\n");

    let config = create_test_config();
    let matcher = FileMatcher::new(&config).unwrap();

    let test_paths: Vec<_> = (0..10000)
        .map(|i| format!("src/services/module-{:04}/api/index.ts", i % 100))
        .collect();

    let iterations = 10;
    let mut durations = Vec::new();

    for i in 0..iterations {
        let start = Instant::now();
        for path in &test_paths {
            black_box(matcher.check_path(Path::new(path)));
        }
        let duration = start.elapsed();
        durations.push(duration);

        if i == 0 {
            println!("  Run {}: {} paths in {:?}", i + 1, test_paths.len(), duration);
        }
    }

    let avg_duration: Duration = durations.iter().sum::<Duration>() / iterations as u32;
    let matches_per_sec = test_paths.len() as f64 / avg_duration.as_secs_f64();

    println!("\n  Average: {:?} for {} paths", avg_duration, test_paths.len());
    println!("  Throughput: {:.0} matches/sec", matches_per_sec);
}

fn bench_full_check() {
    println!("\n=== Full Check Benchmark (Streaming) ===\n");

    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let num_modules = 500;
    let files_per_module = 100;
    let expected_files = num_modules * (2 + files_per_module);

    println!(
        "Creating {} modules ({} files)...",
        num_modules, expected_files
    );

    create_large_directory_structure(root, num_modules, files_per_module);

    let config = create_test_config();
    let matcher = FileMatcher::new(&config).unwrap();
    let walker = Walker::new(root);

    let iterations = 5;
    let mut durations = Vec::new();

    for i in 0..iterations {
        let start = Instant::now();

        let violations = walker.walk_and_process(|path| matcher.check_path(path));

        let duration = start.elapsed();
        durations.push(duration);

        println!(
            "  Run {}: {} violations in {:?}",
            i + 1,
            violations.len(),
            duration
        );
    }

    let avg_duration: Duration = durations.iter().sum::<Duration>() / iterations as u32;

    println!("\n  Average: {:?}", avg_duration);
    println!(
        "  Estimated 500k files: {:.2}s",
        avg_duration.as_secs_f64() * (500_000.0 / expected_files as f64)
    );
}

fn bench_full_check_large() {
    println!("\n=== Large Scale Benchmark (100k files) ===\n");

    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let num_modules = 1000;
    let files_per_module = 100;
    let expected_files = num_modules * (2 + files_per_module);

    println!(
        "Creating {} modules ({} files)...",
        num_modules, expected_files
    );

    create_large_directory_structure(root, num_modules, files_per_module);

    let config = create_test_config();
    let matcher = FileMatcher::new(&config).unwrap();
    let walker = Walker::new(root);

    let iterations = 3;
    let mut durations = Vec::new();

    for i in 0..iterations {
        let start = Instant::now();

        let violations = walker.walk_and_process(|path| matcher.check_path(path));

        let duration = start.elapsed();
        durations.push(duration);

        println!(
            "  Run {}: {} violations in {:?}",
            i + 1,
            violations.len(),
            duration
        );
    }

    let avg_duration: Duration = durations.iter().sum::<Duration>() / iterations as u32;
    let throughput = expected_files as f64 / avg_duration.as_secs_f64();

    println!("\n  Average: {:?}", avg_duration);
    println!("  Throughput: {:.0} files/sec", throughput);
    println!(
        "  Estimated 500k files: {:.2}s",
        avg_duration.as_secs_f64() * (500_000.0 / expected_files as f64)
    );
}

fn bench_full_check_xlarge() {
    println!("\n=== Extra Large Benchmark (200k files) ===\n");

    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let num_modules = 2000;
    let files_per_module = 100;
    let expected_files = num_modules * (2 + files_per_module);

    println!(
        "Creating {} modules ({} files)...",
        num_modules, expected_files
    );

    create_large_directory_structure(root, num_modules, files_per_module);

    let config = create_test_config();
    let matcher = FileMatcher::new(&config).unwrap();
    let walker = Walker::new(root);

    let iterations = 3;
    let mut durations = Vec::new();

    for i in 0..iterations {
        let start = Instant::now();

        let violations = walker.walk_and_process(|path| matcher.check_path(path));

        let duration = start.elapsed();
        durations.push(duration);

        println!(
            "  Run {}: {} violations in {:?}",
            i + 1,
            violations.len(),
            duration
        );
    }

    let avg_duration: Duration = durations.iter().sum::<Duration>() / iterations as u32;
    let throughput = expected_files as f64 / avg_duration.as_secs_f64();

    println!("\n  Average: {:?}", avg_duration);
    println!("  Throughput: {:.0} files/sec", throughput);
    println!(
        "  Estimated 500k files: {:.2}s",
        500_000.0 / throughput
    );
    
    if throughput >= 2_000_000.0 {
        println!("  [PASS] Meets target of 2M files/sec!");
    } else {
        println!("  [INFO] Current: {:.1}% of 2M target", throughput / 2_000_000.0 * 100.0);
    }
}

fn main() {
    println!("repo-lint Performance Benchmarks");
    println!("================================");

    bench_walker_throughput();
    bench_matcher_throughput();
    bench_full_check();
    bench_full_check_large();
    bench_full_check_xlarge();

    println!("\n================================");
    println!("Benchmarks complete.");
}
