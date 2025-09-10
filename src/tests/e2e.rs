#[cfg(test)]
mod e2e_tests {
    use std::process::Command;
    use std::path::Path;
    
    fn run_oxidizr(args: &[&str]) -> (bool, String, String) {
        let output = Command::new("cargo")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args(&["run", "--"])
            .args(args)
            .output()
            .expect("Failed to execute oxidizr-arch");
        
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        (success, stdout, stderr)
    }
    
    #[test]
    #[ignore] // Run with --ignored flag to execute e2e tests
    fn test_help_command() {
        let (success, stdout, _) = run_oxidizr(&["--help"]);
        assert!(success);
        assert!(stdout.contains("oxidizr-arch style coreutils switching"));
        assert!(stdout.contains("enable"));
        assert!(stdout.contains("disable"));
        assert!(stdout.contains("check"));
        assert!(stdout.contains("list-targets"));
    }
    
    #[test]
    #[ignore]
    fn test_check_command() {
        let (success, stdout, _) = run_oxidizr(&["check", "--experiments", "coreutils"]);
        assert!(success);
        // Should show compatibility status
        assert!(stdout.contains("coreutils") || stdout.contains("Compatible"));
    }
    
    #[test]
    #[ignore]
    fn test_list_targets_command() {
        let (success, stdout, _) = run_oxidizr(&["list-targets", "--experiments", "findutils"]);
        assert!(success);
        assert!(stdout.contains("/usr/bin/find"));
        assert!(stdout.contains("/usr/bin/xargs"));
    }
    
    #[test]
    #[ignore]
    fn test_dry_run_enable() {
        let (success, stdout, _) = run_oxidizr(&[
            "enable",
            "--experiments", "sudo-rs",
            "--dry-run",
            "-y"
        ]);
        assert!(success);
        assert!(stdout.contains("[dry-run]") || stdout.contains("Enabled experiment"));
    }
    
    #[test]
    #[ignore]
    fn test_unsupported_distro_check() {
        // This test would need to mock /etc/os-release to simulate unsupported distro
        // Skipping for now as it requires root/container environment
    }
    
    #[test]
    #[ignore]
    fn test_all_experiments_flag() {
        let (success, stdout, _) = run_oxidizr(&["list-targets", "--all"]);
        assert!(success);
        // Should list targets for all three experiments
        assert!(stdout.contains("coreutils"));
        assert!(stdout.contains("findutils"));
        assert!(stdout.contains("sudo-rs"));
    }
}
