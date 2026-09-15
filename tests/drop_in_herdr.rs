use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_herdr"))
        .args(args)
        .env_remove("HERDR_CONFIG_PATH")
        .env_remove("HERDR_SOCKET_PATH")
        .env_remove("HERDR_CLIENT_SOCKET_PATH")
        .output()
        .expect("run herdr CLI")
}

#[test]
fn herdr_command_identifies_superherdr_and_cannot_self_update() {
    let base = env!("CARGO_PKG_VERSION");

    let version = run(&["--version"]);
    assert!(version.status.success());
    let output = String::from_utf8_lossy(&version.stdout);
    let line = output.trim_end();
    // Upstream prints `herdr <version>`; keep those two words and append the Superherdr version.
    let rest = line.strip_prefix(&format!("herdr {base}")).unwrap();
    let (suffix, product) = rest.split_once(" (superherdr ").unwrap();
    assert!(suffix.is_empty() || suffix.starts_with('-'));
    let superherdr_version = product.strip_suffix(')').unwrap();
    let revision = superherdr_version
        .strip_prefix(&format!("{base}."))
        .unwrap();
    assert!(revision.parse::<u32>().unwrap() >= 1);

    let status = run(&["status", "client", "--json"]);
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert!(status["version"].as_str().unwrap().starts_with(base));
    assert_eq!(status["superherdr_version"], superherdr_version);

    let help = run(&["--help"]);
    assert!(help.status.success());
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("Usage: herdr"));
    assert!(help.contains(if cfg!(debug_assertions) {
        "herdr-dev"
    } else {
        "herdr"
    }));
    assert!(!help.contains("superherdr"));

    let completion = run(&["completion", "zsh"]);
    assert!(completion.status.success());
    assert!(String::from_utf8_lossy(&completion.stdout).contains("#compdef herdr"));

    let skill = run(&["--skill"]);
    assert!(skill.status.success());
    let skill = String::from_utf8_lossy(&skill.stdout);
    assert!(skill.contains("name: herdr"));
    assert!(!skill.contains("superherdr"));

    let update = run(&["update"]);
    assert!(!update.status.success());
    assert!(
        String::from_utf8_lossy(&update.stderr).contains("self-update is disabled for Superherdr")
    );
}
