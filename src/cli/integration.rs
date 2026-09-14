use crate::api::schema::IntegrationTarget;

pub(super) fn run_integration_command(args: &[String]) -> std::io::Result<i32> {
    let Some(subcommand) = args.first().map(|arg| arg.as_str()) else {
        print_integration_help();
        return Ok(2);
    };

    match subcommand {
        "install" => integration_install(&args[1..]),
        "uninstall" => integration_uninstall(&args[1..]),
        "status" => integration_status(&args[1..]),
        "help" | "--help" | "-h" => {
            print_integration_help();
            Ok(0)
        }
        _ => {
            print_integration_help();
            Ok(2)
        }
    }
}

fn integration_status(args: &[String]) -> std::io::Result<i32> {
    let outdated_only = match args {
        [] => false,
        [flag] if flag == "--outdated-only" => true,
        _ => {
            eprintln!("usage: superherdr integration status [--outdated-only]");
            return Ok(2);
        }
    };

    if outdated_only {
        crate::integration::print_outdated_update_notice();
        return Ok(0);
    }

    for status in crate::integration::installed_integration_statuses() {
        let target = crate::integration::integration_target_label(status.target);
        let version = match status.installed_version {
            Some(version) => format!("v{version}"),
            None => "legacy".to_string(),
        };
        let state = match status.state {
            crate::integration::IntegrationStatusKind::NotInstalled => "not installed".to_string(),
            crate::integration::IntegrationStatusKind::Current => {
                format!("current ({version})")
            }
            crate::integration::IntegrationStatusKind::Outdated
                if status
                    .installed_version
                    .is_some_and(|installed| installed >= status.expected_version) =>
            {
                format!("needs repair ({version})")
            }
            crate::integration::IntegrationStatusKind::Outdated => {
                format!("outdated ({version} < v{})", status.expected_version)
            }
        };
        println!("{target}: {state} ({})", status.path.display());
    }

    Ok(0)
}

fn integration_install(args: &[String]) -> std::io::Result<i32> {
    let Some(target) = parse_integration_target(args, "install")? else {
        return Ok(2);
    };

    match crate::integration::install_target(target) {
        Ok(messages) => {
            print_integration_messages(messages);
            Ok(0)
        }
        Err(err) => {
            eprintln!("{err}");
            Ok(1)
        }
    }
}

fn integration_uninstall(args: &[String]) -> std::io::Result<i32> {
    let Some(target) = parse_integration_target(args, "uninstall")? else {
        return Ok(2);
    };

    match crate::integration::uninstall_target(target) {
        Ok(messages) => {
            print_integration_messages(messages);
            Ok(0)
        }
        Err(err) => {
            eprintln!("{err}");
            Ok(1)
        }
    }
}

fn print_integration_messages(messages: Vec<String>) {
    for message in messages {
        println!("{message}");
    }
}

fn parse_integration_target(
    args: &[String],
    action: &str,
) -> std::io::Result<Option<IntegrationTarget>> {
    let Some(target) = args.first().map(|arg| arg.as_str()) else {
        eprintln!(
            "usage: superherdr integration {action} <pi|omp|claude|codex|copilot|devin|droid|kimi|opencode|kilo|hermes|qodercli|qwen|cursor|mastracode|grok>"
        );
        return Ok(None);
    };
    if args.len() != 1 {
        eprintln!(
            "usage: superherdr integration {action} <pi|omp|claude|codex|copilot|devin|droid|kimi|opencode|kilo|hermes|qodercli|qwen|cursor|mastracode|grok>"
        );
        return Ok(None);
    }

    let parsed = match target {
        "pi" => IntegrationTarget::Pi,
        "omp" => IntegrationTarget::Omp,
        "claude" => IntegrationTarget::Claude,
        "codex" => IntegrationTarget::Codex,
        "copilot" => IntegrationTarget::Copilot,
        "devin" => IntegrationTarget::Devin,
        "droid" => IntegrationTarget::Droid,
        "kimi" => IntegrationTarget::Kimi,
        "opencode" => IntegrationTarget::Opencode,
        "kilo" => IntegrationTarget::Kilo,
        "hermes" => IntegrationTarget::Hermes,
        "qodercli" => IntegrationTarget::Qodercli,
        "qwen" => IntegrationTarget::Qwen,
        "cursor" => IntegrationTarget::Cursor,
        "mastracode" => IntegrationTarget::Mastracode,
        "antigravity-cli" | "antigravity_cli" => IntegrationTarget::AntigravityCli,
        "grok" => IntegrationTarget::Grok,
        _ => {
            eprintln!("unknown integration target: {target}");
            eprintln!(
                "currently supported: pi, omp, claude, codex, copilot, devin, droid, kimi, opencode, kilo, hermes, qodercli, qwen, cursor, mastracode, antigravity-cli, grok"
            );
            return Ok(None);
        }
    };

    Ok(Some(parsed))
}

fn print_integration_help() {
    eprintln!("superherdr integration commands:");
    eprintln!("  superherdr integration install pi");
    eprintln!("  superherdr integration install omp");
    eprintln!("  superherdr integration install claude");
    eprintln!("  superherdr integration install codex");
    eprintln!("  superherdr integration install copilot");
    eprintln!("  superherdr integration install devin");
    eprintln!("  superherdr integration install droid");
    eprintln!("  superherdr integration install kimi");
    eprintln!("  superherdr integration install opencode");
    eprintln!("  superherdr integration install kilo");
    eprintln!("  superherdr integration install hermes");
    eprintln!("  superherdr integration install qodercli");
    eprintln!("  superherdr integration install qwen");
    eprintln!("  superherdr integration install cursor");
    eprintln!("  superherdr integration install mastracode");
    eprintln!("  superherdr integration install antigravity-cli");
    eprintln!("  superherdr integration install grok");
    eprintln!("  superherdr integration uninstall pi");
    eprintln!("  superherdr integration uninstall omp");
    eprintln!("  superherdr integration uninstall claude");
    eprintln!("  superherdr integration uninstall codex");
    eprintln!("  superherdr integration uninstall copilot");
    eprintln!("  superherdr integration uninstall devin");
    eprintln!("  superherdr integration uninstall droid");
    eprintln!("  superherdr integration uninstall kimi");
    eprintln!("  superherdr integration uninstall opencode");
    eprintln!("  superherdr integration uninstall kilo");
    eprintln!("  superherdr integration uninstall hermes");
    eprintln!("  superherdr integration uninstall qodercli");
    eprintln!("  superherdr integration uninstall qwen");
    eprintln!("  superherdr integration uninstall cursor");
    eprintln!("  superherdr integration uninstall mastracode");
    eprintln!("  superherdr integration uninstall antigravity-cli");
    eprintln!("  superherdr integration uninstall grok");
    eprintln!("  superherdr integration status [--outdated-only]");
}
