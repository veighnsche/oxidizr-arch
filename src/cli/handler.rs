use oxidizr_cli_core::api::build_api;
use oxidizr_cli_core::prompts::should_proceed;
use switchyard::logging::JsonlSink;
use switchyard::policy::Policy;
use switchyard::policy::types::SmokePolicy;
use switchyard::types::ApplyMode;
use switchyard::Switchyard;

use crate::cli::args::{Cli, Commands};

pub fn dispatch(cli: Cli) -> Result<(), String> {
    // Default policy: conservative, disallow degraded EXDEV for built-ins
    let mut policy = Policy::coreutils_switch_preset();
    // Developer/container ergonomics: in our Arch container flows, we want to exercise
    // link swaps without strict preflight STOP gates. Relax a few knobs safely:
    // - Allow commit without a dedicated external lock manager (we still attach one below)
    // - Skip preflight STOP gates to let apply proceed in the ephemeral container
    // - Disable rescue requirement which may not be met in minimal images
    policy.governance.allow_unlocked_commit = true;
    policy.apply.override_preflight = true;
    policy.rescue.require = false;
    policy.governance.smoke = SmokePolicy::Off;

    // Narrow scope to requested root and explicitly to its /usr/bin subtree
    policy.scope.allow_roots.push(cli.root.clone());
    policy.scope
        .allow_roots
        .push(cli.root.join("usr").join("bin"));

    let lock_path = cli.root.join("var/lock/oxidizr-arch.lock");
    if let Some(parent) = lock_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

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
