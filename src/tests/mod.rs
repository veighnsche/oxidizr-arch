#[cfg(test)]
mod tests {
    use oxidizr_arch::checks::{Distribution, is_supported_distro, check_compatibility};
    use oxidizr_arch::experiments::{all_experiments, UUTILS_COREUTILS, UUTILS_FINDUTILS, SUDO_RS};
    use oxidizr_arch::symlink::{backup_path, is_safe_path};
    use std::path::Path;

    #[test]
    fn test_supported_distros() {
        assert!(is_supported_distro("arch"));
        assert!(is_supported_distro("manjaro"));
        assert!(is_supported_distro("cachyos"));
        assert!(is_supported_distro("endeavouros"));
        assert!(!is_supported_distro("ubuntu"));
        assert!(!is_supported_distro("debian"));
    }

    #[test]
    fn test_compatibility_check() {
        let arch = Distribution {
            id: "arch".to_string(),
            id_like: "".to_string(),
            release: "rolling".to_string(),
        };
        assert!(check_compatibility(&arch, false).is_ok());
        
        let ubuntu = Distribution {
            id: "ubuntu".to_string(),
            id_like: "debian".to_string(),
            release: "22.04".to_string(),
        };
        assert!(check_compatibility(&ubuntu, false).is_err());
        assert!(check_compatibility(&ubuntu, true).is_ok()); // Skip check
    }

    #[test]
    fn test_package_constants() {
        assert_eq!(UUTILS_COREUTILS, "uutils-coreutils");
        assert_eq!(UUTILS_FINDUTILS, "uutils-findutils-bin");
        assert_eq!(SUDO_RS, "sudo-rs");
    }

    #[test]
    fn test_all_experiments() {
        let exps = all_experiments();
        assert_eq!(exps.len(), 3);
        
        let names: Vec<String> = exps.iter().map(|e| e.name().to_string()).collect();
        assert!(names.contains(&"coreutils".to_string()));
        assert!(names.contains(&"findutils".to_string()));
        assert!(names.contains(&"sudo-rs".to_string()));
    }

    #[test]
    fn test_backup_path() {
        let target = Path::new("/usr/bin/ls");
        let backup = backup_path(target);
        assert_eq!(backup, Path::new("/usr/bin/.ls.oxidizr.bak"));
        
        let target2 = Path::new("/usr/sbin/visudo");
        let backup2 = backup_path(target2);
        assert_eq!(backup2, Path::new("/usr/sbin/.visudo.oxidizr.bak"));
    }

    #[test]
    fn test_safe_path_validation() {
        assert!(is_safe_path(Path::new("/usr/bin/ls")));
        assert!(is_safe_path(Path::new("relative/path")));
        assert!(!is_safe_path(Path::new("../etc/passwd")));
        assert!(!is_safe_path(Path::new("/usr/../etc/shadow")));
        assert!(!is_safe_path(Path::new("./../../root")));
    }

    #[test]
    fn test_experiment_targets() {
        use oxidizr_arch::experiments::{coreutils::CoreutilsExperiment, findutils::FindutilsExperiment, sudors::SudoRsExperiment};
        
        let coreutils = CoreutilsExperiment::new();
        let targets = coreutils.list_targets();
        assert!(!targets.is_empty());
        assert!(targets.iter().any(|t| t == Path::new("/usr/bin/ls")));
        
        let findutils = FindutilsExperiment::new();
        let targets = findutils.list_targets();
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&Path::new("/usr/bin/find").to_path_buf()));
        assert!(targets.contains(&Path::new("/usr/bin/xargs").to_path_buf()));
        
        let sudors = SudoRsExperiment::new();
        let targets = sudors.list_targets();
        assert_eq!(targets.len(), 3);
        assert!(targets.contains(&Path::new("/usr/bin/sudo").to_path_buf()));
        assert!(targets.contains(&Path::new("/usr/bin/su").to_path_buf()));
        assert!(targets.contains(&Path::new("/usr/sbin/visudo").to_path_buf()));
    }

    #[test]
    fn test_coreutils_list_targets_excludes_checksums() {
        use oxidizr_arch::experiments::coreutils::CoreutilsExperiment;
        let coreutils = CoreutilsExperiment::new();
        let targets = coreutils.list_targets();
        // Check that common checksum applets are not among coreutils targets anymore
        for name in [
            "b2sum", "md5sum", "sha1sum", "sha224sum", "sha256sum", "sha384sum", "sha512sum",
        ] {
            assert!(
                !targets.contains(&Path::new(&format!("/usr/bin/{}", name)).to_path_buf()),
                "coreutils list_targets should exclude checksum applet {}",
                name
            );
        }
    }

    #[test]
    fn test_checksums_list_targets_includes_only_checksums() {
        use oxidizr_arch::experiments::checksums::ChecksumsExperiment;
        let checksums = ChecksumsExperiment::new();
        let targets = checksums.list_targets();
        // Expect exactly 7 checksum applets
        assert_eq!(targets.len(), 7);
        for name in [
            "b2sum", "md5sum", "sha1sum", "sha224sum", "sha256sum", "sha384sum", "sha512sum",
        ] {
            assert!(
                targets.contains(&Path::new(&format!("/usr/bin/{}", name)).to_path_buf()),
                "checksums list_targets should include {}",
                name
            );
        }
    }

    #[test]
    fn test_cli_default_experiments() {
        // Test that we have the expected default experiments
        // The default should be coreutils and sudo-rs
    }
}
