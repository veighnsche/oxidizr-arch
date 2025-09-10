use crate::cli::parser::{Cli, Commands};
use crate::error::Result;
use crate::experiments::{all_experiments, Experiment};
use crate::logging::{audit_event_fields, AuditFields};
use crate::system::{lock, hook, Worker};
use crate::state;
use std::io::{self, Write};

/// Main CLI handler - preserves backward compatibility with original
pub fn handle_cli(cli: Cli) -> Result<()> {
    // Top-level CLI span for context
    let _cli_span = tracing::info_span!(
        "cli",
        command = ?cli.command,
        all = cli.all,
        exp_count = cli.experiments.len(),
        dry_run = cli.dry_run,
        assume_yes = cli.assume_yes
    )
    .entered();
    // Use configured AUR helper (string value from enum)
    let effective_helper = cli.aur_helper.as_helper_str().to_string();

    let worker = Worker::new(
        effective_helper,
        cli.aur_user.clone(),
        cli.dry_run,
        cli.wait_lock,
        cli.package.clone(),
        cli.bin_dir.clone(),
        cli.unified_binary.clone(),
        cli.force_restore_best_effort,
        cli.strict_ownership,
        cli.force_override_untrusted,
        cli.state_dir.clone(),
        cli.log_dir.clone(),
        cli.sudo_smoke_user.clone(),
    );

    let update_lists = !cli.no_update;

    // Configure progress behavior from CLI
    crate::ui::progress::set_disabled(cli.no_progress);

    // Build experiment selection
    let selection: Vec<String> = if cli.all {
        // Will be replaced by all experiments below
        Vec::new()
    } else if !cli.experiments.is_empty() {
        cli.experiments.clone()
    } else if let Some(single) = &cli.experiment {
        vec![single.clone()]
    } else {
        // No implicit defaults: require explicit selection
        tracing::error!("No experiments selected. Use --all or --experiments=<names>");
        return Err(crate::Error::CliMisuse("no experiments selected".into()));
    };

    let all_exps = all_experiments();
    let exps: Vec<Experiment> = if cli.all {
        all_exps
    } else {
        all_exps
            .into_iter()
            .filter(|e| selection.contains(&e.name().to_string()))
            .collect()
    };

    // Orchestration visibility: when both findutils and coreutils are selected, we enable
    // findutils first so GNU checksum tools remain available for AUR builds before any flipping.
    let names: Vec<String> = exps.iter().map(|e| e.name().to_string()).collect();
    let has_core = names.iter().any(|n| n == "coreutils");
    let has_find = names.iter().any(|n| n == "findutils");
    if has_core && has_find {
        tracing::info!(step = "orchestration", "enable findutils before coreutils");
    }

    if exps.is_empty() {
        tracing::error!("No experiments matched the selection");
        return Err(crate::Error::CliMisuse(
            "no experiments matched selection".into(),
        ));
    }

    match cli.command {
        Commands::Enable => {
            // Enforce single-instance lock for mutating command
            let _lock_guard = lock::acquire()?;
            if !cli.dry_run {
                enforce_root()?;
            }
            if !cli.assume_yes && !confirm("Enable and switch to Rust replacements?")? {
                return Ok(());
            }
            for (idx, e) in exps.iter().enumerate() {
                tracing::info!(step = "enable_sequence", idx = idx + 1, total = exps.len(), experiment = %e.name());
                e.enable(
                    &worker,
                    cli.assume_yes,
                    update_lists,
                    cli.no_compatibility_check,
                )?;
                tracing::info!(event = "enabled", experiment = %e.name());
                let _ = audit_event_fields(
                    "cli",
                    "enabled",
                    "success",
                    &AuditFields { target: Some(e.name().to_string()), ..Default::default() },
                );
            }
            // Emit final state report
            if !cli.dry_run { let _ = state::write_state_report(worker.state_dir_override.as_deref(), worker.log_dir_override.as_deref()); }
        }
        Commands::Disable => {
            let _lock_guard = lock::acquire()?;
            if !cli.dry_run {
                enforce_root()?;
            }
            // Restore only; never uninstall in Disable
            for e in &exps {
                e.disable(&worker, cli.assume_yes, update_lists)?;
                tracing::info!(event = "disabled", experiment = %e.name());
                let _ = audit_event_fields(
                    "cli",
                    "disabled",
                    "success",
                    &AuditFields { target: Some(e.name().to_string()), ..Default::default() },
                );
            }
            if !cli.dry_run { let _ = state::write_state_report(worker.state_dir_override.as_deref(), worker.log_dir_override.as_deref()); }
        }
        Commands::Remove => {
            let _lock_guard = lock::acquire()?;
            if !cli.dry_run {
                enforce_root()?;
            }
            for e in &exps {
                e.remove(&worker, cli.assume_yes, update_lists)?;
                tracing::info!(event = "removed_and_restored", experiment = %e.name());
                let _ = audit_event_fields(
                    "cli",
                    "removed_and_restored",
                    "success",
                    &AuditFields { target: Some(e.name().to_string()), ..Default::default() },
                );
            }
            if !cli.dry_run { let _ = state::write_state_report(worker.state_dir_override.as_deref(), worker.log_dir_override.as_deref()); }
        }
        Commands::Check => {
            let distro = worker.distribution()?;
            let mut any_incompatible = false;
            for e in &exps {
                let ok = e.check_compatible(&distro)?;
                println!("{}\tCompatible: {}", e.name(), ok);
                if !ok { any_incompatible = true; }
                tracing::info!(event = "compatibility_check", experiment = %e.name(), distro = %distro.id, compatible = ok, "compatibility: {} -> {}", e.name(), ok);
            }
            if any_incompatible {
                return Err(crate::Error::Incompatible(format!("one or more experiments incompatible with {}", worker.distribution()?.id)));
            }
        }
        Commands::ListTargets => {
            for e in &exps {
                for t in e.list_targets() {
                    println!("{}\t{}", e.name(), t.display());
                    tracing::info!(event = "list_target", experiment = %e.name(), target = %t.display().to_string());
                }
            }
        }
        Commands::RelinkManaged => {
            let _lock_guard = lock::acquire()?;
            if !cli.dry_run { enforce_root()?; }
            // Relink previously managed experiments from persisted state
            crate::experiments::relink_managed(&worker, /*assume_yes=*/ true, /*update_lists=*/ false)?;
            if !cli.dry_run { let _ = state::write_state_report(worker.state_dir_override.as_deref(), worker.log_dir_override.as_deref()); }
        }
        Commands::InstallHook => {
            let _lock_guard = lock::acquire()?;
            if !cli.dry_run { enforce_root()?; }
            if cli.dry_run {
                let path = hook::hook_path();
                let body = hook::hook_body();
                println!("[dry-run] Would install pacman hook at {}\n\n{}", path.display(), body);
            } else {
                let path = hook::install_pacman_hook()?;
                println!("Installed pacman hook at {}", path.display());
            }
        }
    }

    Ok(())
}

fn enforce_root() -> Result<()> {
    #[cfg(unix)]
    {
        use nix::unistd::Uid;
        if !Uid::effective().is_root() {
            return Err(crate::Error::RootRequired);
        }
    }
    Ok(())
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{} [y/N]: ", prompt);
    io::stdout().flush().ok();
    let mut s = String::new();
    if io::stdin().read_line(&mut s).is_err() {
        return Ok(false);
    }
    let ans = s.trim().to_ascii_lowercase();
    Ok(ans == "y" || ans == "yes")
}
