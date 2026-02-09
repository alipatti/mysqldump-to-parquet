use std::fs;
use std::path::Path;
use std::process::Command;

/// Test with the Wikipedia redirect dump
/// Run with: cargo test --test integration -- --ignored --nocapture
#[test]
#[ignore]
fn test_wikipedia_redirect_dump() {
    let dump_url = "https://dumps.wikimedia.org/enwiki/latest/enwiki-latest-redirect.sql.gz";
    let dump_path = "/tmp/enwiki-latest-redirect.sql.gz";
    let output_dir = "/tmp/enwiki-redirect-parquet";

    // Download if not exists
    if !Path::new(dump_path).exists() {
        eprintln!("Downloading {}...", dump_url);
        let status = Command::new("curl")
            .args(["-L", "-o", dump_path, dump_url])
            .status()
            .expect("Failed to run curl");
        assert!(status.success(), "Failed to download dump file");
    }

    // Clean output directory
    let _ = fs::remove_dir_all(output_dir);
    fs::create_dir_all(output_dir).unwrap();

    // Build the binary
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .expect("Failed to build");
    assert!(status.success(), "Failed to build release binary");

    // Run the conversion
    eprintln!("Converting {} to parquet...", dump_path);
    let start = std::time::Instant::now();
    let status = Command::new("cargo")
        .args(["run", "--release", "--", "-o", output_dir, dump_path])
        .status()
        .expect("Failed to run conversion");
    let elapsed = start.elapsed();
    eprintln!("Conversion completed in {:?}", elapsed);

    assert!(status.success(), "Conversion failed");

    // Check output
    let output_path = Path::new(output_dir);
    assert!(output_path.exists(), "Output directory not created");

    // Find parquet files
    let parquet_files: Vec<_> = fs::read_dir(output_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "parquet").unwrap_or(false))
        .collect();

    assert!(!parquet_files.is_empty(), "No parquet files created");

    for entry in &parquet_files {
        let file_path = entry.path();
        let file_name = file_path.file_name().unwrap().to_str().unwrap();
        let file_size = entry.metadata().unwrap().len();

        eprintln!(
            "Table '{}': {} MB",
            file_name,
            file_size / 1024 / 1024
        );
    }

    eprintln!("Test passed!");
}

/// Small inline test to verify basic functionality
#[test]
fn test_small_dump() {
    let sql = r#"
CREATE TABLE `users` (
  `id` bigint NOT NULL,
  `name` varchar(255) NOT NULL,
  `email` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

INSERT INTO `users` VALUES (1,'Alice','alice@example.com'),(2,'Bob',NULL),(3,'Charlie','charlie@example.com');
INSERT INTO `users` VALUES (4,'David','david@example.com'),(5,'Eve','eve@example.com');

CREATE TABLE `posts` (
  `id` bigint NOT NULL,
  `user_id` bigint NOT NULL,
  `title` varchar(255) NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

INSERT INTO `posts` VALUES (1,1,'Hello World'),(2,1,'Second Post'),(3,2,'Bobs Post');
"#;

    let temp_dir = tempfile::tempdir().unwrap();
    let sql_path = temp_dir.path().join("test.sql");
    let output_dir = temp_dir.path().join("output");

    fs::write(&sql_path, sql).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    // Run the conversion
    let status = Command::new("cargo")
        .args([
            "run",
            "--",
            "-o",
            output_dir.to_str().unwrap(),
            sql_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run conversion");

    assert!(status.success(), "Conversion failed");

    // Check users table
    let users_file = output_dir.join("users.parquet");
    assert!(users_file.exists(), "users.parquet not created");

    // Check posts table
    let posts_file = output_dir.join("posts.parquet");
    assert!(posts_file.exists(), "posts.parquet not created");
}
