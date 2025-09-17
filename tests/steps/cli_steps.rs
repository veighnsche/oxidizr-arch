use cucumber::when;
use shlex::Shlex;

use crate::world::World;

#[when(regex = r"^I run `oxidizr-arch (.+)`$")]
pub async fn i_run_oxidizr_arch(world: &mut World, cmd: String) {
    let args: Vec<String> = Shlex::new(&cmd).collect();
    let mut final_args: Vec<String> = Vec::new();

    // Inject --root if missing
    if !args.iter().any(|a| a == "--root") {
        final_args.push("--root".into());
        let root = world.ensure_root().to_path_buf();
        final_args.push(root.display().to_string());
    }

    // Copy original args and inject offline/use-local when artifact exists for `use` commands
    final_args.extend(args.clone());
    if args.iter().any(|s| s == "use") {
        if let Some(rel) = world.artifact_path.clone() {
            if !args.iter().any(|a| a == "--use-local") {
                final_args.push("--offline".into());
                final_args.push("--use-local".into());
                let abs = world.under_root(rel);
                final_args.push(abs.display().to_string());
            }
        }
    }

    let out = world.run_cli(final_args);
    world.last_output = Some(out);
}
