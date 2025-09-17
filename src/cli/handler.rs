use oxidizr_cli_core::api::build_api;
use oxidizr_cli_core::prompts::should_proceed;
use switchyard::logging::JsonlSink;
use switchyard::policy::Policy;
use switchyard::types::ApplyMode;
use switchyard::Switchyard;

use crate::cli::args::{Cli, Commands};

pub fn dispatch(cli: Cli) -> Result<(), String> {
    // Default policy: conservative, disallow degraded EXDEV for built-ins
    let mut policy = Policy::coreutils_switch_preset();

    // Narrow scope to requested root
    policy.scope.allow_roots.push(cli.root.clone());

    let lock_path = cli.root.join("var/lock/oxidizr-arch.lock");

    let api: Switchyard<JsonlSink, JsonlSink> = build_api(policy, lock_path);

    let apply_mode = if cli.commit {
        ApplyMode::Commit
    } else {
        ApplyMode::DryRun
    };

    match cli.command {
        Commands::Use {
            package,
            offline,
            use_local,
        } => {
            if matches!(apply_mode, ApplyMode::Commit) {
                if !should_proceed(cli.assume_yes, &cli.root) {
                    return Err("aborted by user".to_string());
                }
            }
            crate::commands::r#use::exec(&api, &cli.root, package, offline, use_local, apply_mode)
        }
        Commands::Restore {
            package,
            all,
            keep_replacements,
        } => {
            if matches!(apply_mode, ApplyMode::Commit) {
                if !should_proceed(cli.assume_yes, &cli.root) {
                    return Err("aborted by user".to_string());
                }
            }
            crate::commands::restore::exec(
                &api,
                &cli.root,
                package,
                all,
                keep_replacements,
                apply_mode,
                cli.assume_yes,
            )
        }
        Commands::Status { json } => crate::commands::status::exec(&cli.root, json),
        Commands::Doctor { json } => crate::commands::doctor::exec(&cli.root, json),
        Commands::Replace { package, all } => crate::commands::replace::exec(
            &api,
            &cli.root,
            package,
            all,
            apply_mode,
            cli.assume_yes,
        ),
        Commands::Completions { shell } => crate::cli::completions::emit(shell),
    }
}
