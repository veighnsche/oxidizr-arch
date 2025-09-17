use clap::CommandFactory;
use clap_complete::shells::{Bash, Fish, Zsh};

use crate::cli::args::{Cli, Shell};

pub fn emit(shell: Shell) -> Result<(), String> {
    let mut cmd = Cli::command();
    match shell {
        Shell::Bash => {
            clap_complete::generate(Bash, &mut cmd, "oxidizr-arch", &mut std::io::stdout())
        }
        Shell::Zsh => {
            clap_complete::generate(Zsh, &mut cmd, "oxidizr-arch", &mut std::io::stdout())
        }
        Shell::Fish => {
            clap_complete::generate(Fish, &mut cmd, "oxidizr-arch", &mut std::io::stdout())
        }
    }
    Ok(())
}
