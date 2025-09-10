use clap::Parser;

fn main() {
    // Parse CLI arguments first so logging can honor dry-run (gates audit sink writes)
    let cli = oxidizr_arch::cli::Cli::parse();
    if cli.dry_run {
        // Signal to logging init to disable audit file sink during dry-run
        std::env::set_var("OXIDIZR_DRY_RUN", "1");
    }
    // Initialize structured logging (tracing) with VERBOSE 0..3 mapping.
    oxidizr_arch::logging::init_logging();

    // Handle command and execute
    if let Err(e) = oxidizr_arch::cli::handle_cli(cli) {
        let code = e.exit_code();
        tracing::error!(error=%e, exit_code=code, "fatal_error");
        std::process::exit(code);
    }
}
