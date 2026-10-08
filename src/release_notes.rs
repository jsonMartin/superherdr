use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const PENDING_RELEASE_NOTES_PATH: &str = "release-notes.json";

// Embedded at build time so the What's New history view works in installed
// builds with no checkout, no network, and no config-directory state.
pub const EMBEDDED_CHANGELOG: &str = include_str!("../CHANGELOG.md");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseNotes {
    pub version: String,
    pub body: String,
    pub preview: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredReleaseNotes {
    version: String,
    body: String,
    #[serde(default = "default_show_on_startup")]
    show_on_startup: bool,
}

fn default_show_on_startup() -> bool {
    true
}

pub fn pending_path() -> PathBuf {
    let mut path = crate::config::config_path();
    path.set_file_name(PENDING_RELEASE_NOTES_PATH);
    path
}

pub fn save_pending(version: &str, body: &str) -> std::io::Result<()> {
    save_pending_to_path(&pending_path(), version, body)
}

fn save_pending_to_path(path: &Path, version: &str, body: &str) -> std::io::Result<()> {
    let body = normalize_body(body);
    if body.is_empty() {
        return clear_pending_at(path);
    }

    write_stored_to_path(
        path,
        &StoredReleaseNotes {
            version: version.to_string(),
            body,
            show_on_startup: true,
        },
    )
}

fn write_stored_to_path(path: &Path, stored: &StoredReleaseNotes) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(stored).map_err(std::io::Error::other)?;
    let tmp_path = path.with_extension(format!("json.tmp.{}", std::process::id()));
    fs::write(&tmp_path, json)?;
    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }
    Ok(())
}

fn load_stored_from_path(path: &Path) -> Option<StoredReleaseNotes> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn load_latest() -> Option<ReleaseNotes> {
    load_latest_from_path(&pending_path(), &crate::build_info::version())
}

fn load_latest_from_path(path: &Path, current_version: &str) -> Option<ReleaseNotes> {
    let stored = load_stored_from_path(path)?;
    release_notes_from_stored(stored, current_version)
}

fn release_notes_from_stored(
    stored: StoredReleaseNotes,
    current_version: &str,
) -> Option<ReleaseNotes> {
    let body = normalize_body(&stored.body);
    if body.is_empty() {
        return None;
    }

    let preview = match (
        crate::update::Version::parse(&stored.version),
        crate::update::Version::parse(current_version),
    ) {
        (Some(stored_version), Some(current_version)) => stored_version > current_version,
        _ => false,
    };

    Some(ReleaseNotes {
        preview,
        version: stored.version,
        body,
    })
}

pub fn mark_current_version_seen() -> std::io::Result<()> {
    mark_current_version_seen_at(&pending_path(), &crate::build_info::version())
}

fn mark_current_version_seen_at(path: &Path, current_version: &str) -> std::io::Result<()> {
    let Some(mut stored) = load_stored_from_path(path) else {
        return Ok(());
    };
    if stored.version != current_version || !stored.show_on_startup {
        return Ok(());
    }
    stored.show_on_startup = false;
    write_stored_to_path(path, &stored)
}

fn clear_pending_at(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        fs::remove_file(path)
    } else {
        Ok(())
    }
}

pub fn load_preview_from_local_changelog(version: &str) -> Option<ReleaseNotes> {
    let path = Path::new("CHANGELOG.md");
    let content = fs::read_to_string(path).ok()?;
    let body = extract_version_section(&content, version)?;
    Some(ReleaseNotes {
        version: version.to_string(),
        body: normalize_body(&body),
        preview: true,
    })
}

fn extract_version_section(content: &str, version: &str) -> Option<String> {
    let header = format!("## [{version}]");
    let mut collecting = false;
    let mut lines = Vec::new();

    for line in content.lines() {
        if !collecting {
            if line.starts_with(&header) {
                collecting = true;
            }
            continue;
        }

        if line.starts_with("## [") || line.starts_with("### Inherited from Herdr") {
            break;
        }

        lines.push(line);
    }

    let body = lines.join("\n").trim().to_string();
    (!body.is_empty()).then_some(body)
}

/// The merged What's New document: released Superherdr sections first
/// (newest first, `Unreleased` omitted) followed by the inherited Herdr
/// sections. Version headings are restyled to `###` so the release-notes
/// renderer styles them, and inherited sections keep their `### Inherited
/// from Herdr <version>` headings.
pub fn full_history_body() -> String {
    let mut lines = Vec::new();
    let mut started = false;
    let mut skipping_unreleased = false;

    for line in EMBEDDED_CHANGELOG.lines() {
        if let Some(rest) = line.strip_prefix("## [") {
            started = true;
            skipping_unreleased = rest.contains("Unreleased");
            if skipping_unreleased {
                continue;
            }
            let version = rest.split(']').next().unwrap_or(rest).trim();
            lines.push(format!("### Superherdr {version}"));
            continue;
        }
        if !started || skipping_unreleased {
            continue;
        }
        lines.push(line.to_string());
    }

    normalize_body(&lines.join("\n"))
}

pub fn normalize_body(body: &str) -> String {
    body.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_version_section() {
        let changelog = "# Changelog\n\n## [0.2.3] - 2026-03-31\n\n### Changed\n- One\n\n## [0.2.2] - 2026-03-30\n\n### Fixed\n- Two\n";
        assert_eq!(
            extract_version_section(changelog, "0.2.3").as_deref(),
            Some("### Changed\n- One")
        );
    }

    #[test]
    fn version_extraction_excludes_a_following_inherited_section() {
        let changelog = "# Changelog\n\n## [0.2.3] - 2026-03-31\n\n### Changed\n- One\n\n### Inherited from Herdr 0.9.3\n\nFixed:\n- Upstream fix\n";
        let body = extract_version_section(changelog, "0.2.3").expect("section body");
        assert_eq!(body, "### Changed\n- One");
        assert!(!body.contains("Inherited"));
    }

    #[test]
    fn extraction_of_the_last_released_section_excludes_inherited_sections() {
        let last_version = EMBEDDED_CHANGELOG
            .lines()
            .rfind(|line| line.starts_with("## [") && !line.contains("Unreleased"))
            .and_then(|line| {
                line.strip_prefix("## [")
                    .and_then(|rest| rest.split(']').next())
            })
            .map(str::to_string)
            .expect("a released section");
        let body = extract_version_section(EMBEDDED_CHANGELOG, &last_version)
            .expect("last released section");
        assert!(
            body.contains("### Changed")
                || body.contains("### Added")
                || body.contains("### Fixed")
        );
        assert!(!body.contains("Inherited from Herdr"));
    }

    #[test]
    fn full_history_starts_at_the_newest_released_superherdr_section() {
        let body = full_history_body();
        assert!(!body.contains("Unreleased"), "{body}");
        assert!(body.starts_with("### Superherdr "), "{body}");
    }

    #[test]
    fn full_history_lists_superherdr_sections_before_inherited_sections() {
        let body = full_history_body();
        let first_inherited = body
            .find("### Inherited from Herdr")
            .expect("inherited sections");
        let last_superherdr = body.rfind("### Superherdr ").expect("superherdr sections");
        assert!(
            last_superherdr < first_inherited,
            "superherdr sections must come before inherited sections"
        );
    }

    #[test]
    fn full_history_lists_inherited_sections_newest_first() {
        let body = full_history_body();
        let expected: Vec<String> = EMBEDDED_CHANGELOG
            .lines()
            .filter_map(|line| line.strip_prefix("### Inherited from Herdr "))
            .map(str::trim)
            .map(str::to_string)
            .collect();
        assert!(!expected.is_empty(), "changelog has inherited sections");
        let mut last_index = None;
        for version in &expected {
            let marker = format!("### Inherited from Herdr {version}");
            let index = body.find(&marker).unwrap_or_else(|| {
                panic!("history body is missing the inherited section for {version}")
            });
            if let Some(previous) = last_index {
                assert!(index > previous, "inherited sections must be newest first");
            }
            last_index = Some(index);
        }
    }

    #[test]
    fn preserves_headings() {
        assert_eq!(
            normalize_body("### Changed\n- One\n\n### Fixed\n- Two"),
            "### Changed\n- One\n\n### Fixed\n- Two"
        );
    }

    #[test]
    fn load_latest_keeps_future_version_previewable_before_restart() {
        let path = std::env::temp_dir().join(format!(
            "herdr-release-notes-{}-{}.json",
            std::process::id(),
            "preview"
        ));
        let _ = clear_pending_at(&path);
        save_pending_to_path(&path, "0.3.2", "### Changed\n- One").unwrap();

        let notes = load_latest_from_path(&path, "0.3.1").expect("latest notes");
        assert_eq!(notes.version, "0.3.2");
        assert_eq!(notes.body, "### Changed\n- One");
        assert!(notes.preview);

        clear_pending_at(&path).unwrap();
    }

    #[test]
    fn load_latest_does_not_mark_older_saved_version_as_preview() {
        let path = std::env::temp_dir().join(format!(
            "herdr-release-notes-{}-{}.json",
            std::process::id(),
            "stale"
        ));
        let _ = clear_pending_at(&path);
        save_pending_to_path(&path, "0.3.0", "### Changed\n- One").unwrap();

        let notes = load_latest_from_path(&path, "0.3.1").expect("latest notes");
        assert_eq!(notes.version, "0.3.0");
        assert_eq!(notes.body, "### Changed\n- One");
        assert!(!notes.preview);

        clear_pending_at(&path).unwrap();
    }

    #[test]
    fn marking_current_version_seen_preserves_latest_notes() {
        let path = std::env::temp_dir().join(format!(
            "herdr-release-notes-{}-{}.json",
            std::process::id(),
            "seen"
        ));
        let _ = clear_pending_at(&path);
        save_pending_to_path(&path, "0.3.1", "### Changed\n- One").unwrap();

        mark_current_version_seen_at(&path, "0.3.1").unwrap();

        let stored = load_stored_from_path(&path).expect("stored notes");
        assert!(!stored.show_on_startup);
        let latest = load_latest_from_path(&path, "0.3.1").expect("latest notes");
        assert_eq!(latest.version, "0.3.1");
        assert!(!latest.preview);

        clear_pending_at(&path).unwrap();
    }

    #[test]
    fn legacy_notes_without_show_on_startup_remain_available_as_latest() {
        let path = std::env::temp_dir().join(format!(
            "herdr-release-notes-{}-{}.json",
            std::process::id(),
            "legacy"
        ));
        let _ = clear_pending_at(&path);
        fs::write(
            &path,
            "{\n  \"version\": \"0.3.1\",\n  \"body\": \"### Changed\\n- One\"\n}",
        )
        .unwrap();

        assert!(load_latest_from_path(&path, "0.3.1").is_some());

        clear_pending_at(&path).unwrap();
    }
}
