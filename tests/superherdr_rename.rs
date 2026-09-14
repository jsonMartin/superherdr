use std::process::Command;

#[test]
fn renamed_cli_is_isolated_and_cannot_install_upstream_updates() {
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_superherdr"))
            .args(args)
            .env_remove("HERDR_CONFIG_PATH")
            .env_remove("HERDR_SOCKET_PATH")
            .env_remove("HERDR_CLIENT_SOCKET_PATH")
            .output()
            .expect("run Superherdr CLI")
    };
    let version = run(&["--version"]);
    assert!(version.status.success());
    let expected = format!("superherdr {}", env!("CARGO_PKG_VERSION"));
    let output = String::from_utf8_lossy(&version.stdout);
    let suffix = output.trim_end().strip_prefix(&expected).unwrap();
    // Preview builds append a channel suffix to the same package version.
    assert!(suffix.is_empty() || suffix.starts_with('-'));

    let help = run(&["--help"]);
    assert!(help.status.success());
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("Usage: superherdr"));
    assert!(help.contains(if cfg!(debug_assertions) {
        "superherdr-dev"
    } else {
        "superherdr"
    }));

    let completion = run(&["completion", "zsh"]);
    assert!(completion.status.success());
    assert!(String::from_utf8_lossy(&completion.stdout).contains("#compdef superherdr"));

    let skill = run(&["--skill"]);
    assert!(skill.status.success());
    let skill = String::from_utf8_lossy(&skill.stdout);
    assert!(skill.contains("name: superherdr"));
    assert!(!skill.contains("`herdr`"));

    let update = run(&["update"]);
    assert!(!update.status.success());
    assert!(
        String::from_utf8_lossy(&update.stderr).contains("self-update is disabled for Superherdr")
    );
}
