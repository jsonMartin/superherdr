# herdr task runner
set windows-shell := ["cmd.exe", "/d", "/s", "/c"]

python := if os() == "windows" { "python" } else { "python3" }

# Run tests
test:
    cargo nextest run --locked --status-level fail --final-status-level fail --failure-output final --success-output never
    just maintenance-test
    just ui-hot-path-architecture-test
    just integration-assets-test

# Run repository maintenance contract tests
maintenance-test:
    {{python}} -m unittest scripts.test_agent_detection_manifest_check scripts.test_hermes_integration_asset scripts.test_package_windows_conpty scripts.test_sync_upstream scripts.test_unix_installer scripts.test_vendor_libghostty_vt scripts.test_vendor_portable_pty

# Run one nextest filter, e.g. `just test-one codex_stale_working`
test-one filter:
    cargo nextest run --locked "{{filter}}" --status-level fail --final-status-level fail --failure-output final --success-output never

# Enforce deterministic UI hot-path architecture boundaries
ui-hot-path-architecture-test:
    {{python}} -m unittest scripts.test_ui_hot_path_architecture

# Run fast local lint checks
[unix]
lint:
    cargo fmt --check
    cargo clippy --all-targets --locked -- -D warnings

[script("powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File")]
[windows]
lint:
    & .\scripts\windows_check.ps1 -Mode lint

# Run PR CI checks
ci filter='all()': lint
    cargo nextest run --locked -E "{{filter}}" --status-level fail --final-status-level slow --failure-output final --success-output never
    just maintenance-test
    just ui-hot-path-architecture-test
    just integration-assets-test

# Run Windows target lint from Unix/macOS to catch cfg(windows) compile and clippy failures before CI
[unix]
windows-lint:
    rustup target add x86_64-pc-windows-msvc
    LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --locked --target x86_64-pc-windows-msvc -- -D warnings

# Check formatting + run unit tests + OpenSpec validation
[unix]
check: ci
    just spec-check

[script("powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File")]
[windows]
check:
    & .\scripts\windows_check.ps1 -Mode check

# Install repo-local git hooks
install-hooks:
    git config core.hooksPath .githooks
    chmod +x .githooks/pre-commit
    chmod +x .githooks/commit-msg
    @echo "installed git hooks from .githooks"

# Build release binary
build:
    cargo build --release --locked

# Non-gating full-render scaling profile for background workspaces and active panes
bench-render-scale:
    cargo test --release --locked --bin herdr render_scale_profile -- --ignored --nocapture --test-threads=1

# ~3-5 minute CPU comparison; downloads stable unless HERDR_PERF_BASELINE_BIN is set
bench-release-smoke:
    cargo build --release --locked
    scripts/release_perf_smoke.sh "${CARGO_TARGET_DIR:-target}/release/herdr"

# Test bundled agent integration assets
integration-assets-test:
    bun test src/integration/assets/herdr-agent-state.test.ts
    bun test src/integration/assets/opencode/herdr-agent-state.test.ts
    bun test src/integration/assets/opencode/herdr-tui-session.test.ts

# Build the vendored libghostty-vt source dist
build-libghostty-vt:
    scripts/build_vendored_libghostty_vt.sh

# Validate OpenSpec baseline specs and active changes
spec-check:
    bunx --bun @fission-ai/openspec@1.13.0 validate --all --strict

# Merge an upstream Herdr release tag, applying the fork's exclusions (usage: just sync-upstream v0.9.1)
sync-upstream tag:
    scripts/sync_upstream.sh {{tag}}

# Fail if paths removed from the fork on purpose have been reintroduced
upstream-exclusions-check:
    scripts/sync_upstream.sh --check

# Print default config
default-config:
    cargo run --release --locked -- --default-config
