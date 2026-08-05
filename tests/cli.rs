use std::{fs, path::Path, process::Command};

use tempfile::TempDir;

fn rust_project() -> TempDir {
    let directory = tempfile::tempdir().expect("temporary project");
    fs::create_dir_all(directory.path().join("src")).expect("src directory");
    fs::write(
        directory.path().join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("Cargo.toml");
    fs::write(directory.path().join("src/main.rs"), "fn main() {}\n").expect("main.rs");
    directory
}

fn toolbox(project: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_toolbox"));
    command.arg("--project").arg(project);
    command
}

#[test]
fn dry_run_does_not_write() {
    let project = rust_project();
    let output = toolbox(project.path())
        .args(["add", "rabbitmq", "--dry-run"])
        .output()
        .expect("run toolbox");

    assert!(output.status.success());
    assert!(!project.path().join(".toolbox").exists());
    assert!(!project.path().join(".env.example").exists());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Dry-run"));
}

#[test]
fn add_is_idempotent_and_preserves_existing_env() {
    let project = rust_project();
    fs::write(
        project.path().join(".env.example"),
        "RABBITMQ_URL=amqps://example.invalid/%2f\n",
    )
    .expect("existing env");

    let first = toolbox(project.path())
        .args(["add", "rabbitmq"])
        .output()
        .expect("first add");
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );

    let second = toolbox(project.path())
        .args(["add", "rabbitmq"])
        .output()
        .expect("second add");
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );

    let env = fs::read_to_string(project.path().join(".env.example")).expect("generated env");
    assert!(env.contains("RABBITMQ_URL=amqps://example.invalid/%2f"));
    assert_eq!(env.matches("RABBITMQ_URL=").count(), 1);
    assert!(
        project
            .path()
            .join(".toolbox/rabbitmq/compose.yaml")
            .is_file()
    );
    assert!(
        project
            .path()
            .join(".toolbox/rabbitmq/check/src/main.rs")
            .is_file()
    );
    assert!(String::from_utf8_lossy(&second.stdout).contains("unchanged"));
}

#[test]
fn edited_generated_file_requires_force() {
    let project = rust_project();
    let first = toolbox(project.path())
        .args(["add", "rabbitmq"])
        .output()
        .expect("first add");
    assert!(first.status.success());

    let compose = project.path().join(".toolbox/rabbitmq/compose.yaml");
    fs::write(&compose, "# edited locally\n").expect("edit generated compose");

    let refused = toolbox(project.path())
        .args(["add", "rabbitmq"])
        .output()
        .expect("refused add");
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--force"));

    let forced = toolbox(project.path())
        .args(["add", "rabbitmq", "--force"])
        .output()
        .expect("forced add");
    assert!(forced.status.success());
    let restored = fs::read_to_string(compose).expect("restored compose");
    assert!(restored.contains("rabbitmq:4.3.3-management"));
}

#[test]
fn up_refuses_a_remote_url_before_calling_docker() {
    let project = rust_project();
    let added = toolbox(project.path())
        .args(["add", "rabbitmq"])
        .output()
        .expect("add rabbitmq");
    assert!(added.status.success());

    let output = toolbox(project.path())
        .arg("up")
        .env("RABBITMQ_URL", "amqps://rabbitmq.example/%2f")
        .output()
        .expect("refused remote up");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("uniquement le provider local"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Démarrage"));
}

#[test]
fn redis_add_is_idempotent_and_coexists_with_rabbitmq() {
    let project = rust_project();

    for service in ["rabbitmq", "redis"] {
        let output = toolbox(project.path())
            .args(["add", service])
            .output()
            .expect("add service");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let env = fs::read_to_string(project.path().join(".env.example")).expect("generated env");
    assert_eq!(env.matches("RABBITMQ_URL=").count(), 1);
    assert_eq!(env.matches("REDIS_URL=").count(), 1);
    assert!(project.path().join(".toolbox/redis/compose.yaml").is_file());

    for service in ["rabbitmq", "redis"] {
        let output = toolbox(project.path())
            .args(["add", service])
            .output()
            .expect("repeat add service");
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("unchanged"));
        assert_eq!(
            fs::read_to_string(project.path().join(".env.example")).unwrap(),
            env
        );
    }
}

#[test]
fn redis_up_refuses_a_remote_url_before_calling_docker() {
    let project = rust_project();
    let added = toolbox(project.path())
        .args(["add", "redis"])
        .output()
        .expect("add redis");
    assert!(added.status.success());

    let output = toolbox(project.path())
        .args(["up", "redis"])
        .env("REDIS_URL", "rediss://cache.example/")
        .output()
        .expect("refused remote up");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("uniquement le provider local"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Démarrage de Redis"));
}

#[test]
fn up_without_an_installed_service_explains_how_to_start() {
    let project = rust_project();
    let output = toolbox(project.path())
        .arg("up")
        .output()
        .expect("empty up");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("aucun service installé"));
}
