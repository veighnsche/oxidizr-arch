#[cfg(feature = "bdd")]
#[derive(Debug, Default, cucumber::World)]
pub struct World {
    pub root: Option<tempfile::TempDir>,
    pub last_output: Option<std::process::Output>,
    pub artifact_path: Option<std::path::PathBuf>,
    pub envs: Vec<(String, String)>,
    pub pacman_lock: Option<std::fs::File>,
}

#[cfg(feature = "bdd")]
impl World {
    pub fn ensure_root(&mut self) -> &std::path::Path {
        if self.root.is_none() {
            self.root = Some(tempfile::TempDir::new().expect("temp root"));
        }
        self.root.as_ref().unwrap().path()
    }

    pub fn root_pathbuf(&mut self) -> std::path::PathBuf {
        self.ensure_root().to_path_buf()
    }

    pub fn under_root<P: AsRef<std::path::Path>>(&mut self, rel: P) -> std::path::PathBuf {
        let r = self.ensure_root().to_path_buf();
        let rel = rel.as_ref();
        if rel.is_absolute() {
            r.join(rel.strip_prefix("/").unwrap())
        } else {
            r.join(rel)
        }
    }

    pub fn ensure_dir<P: AsRef<std::path::Path>>(&mut self, rel: P) {
        let p = self.under_root(rel);
        std::fs::create_dir_all(&p).expect("create dir under root");
    }

    pub fn write_file<P: AsRef<std::path::Path>>(&mut self, rel: P, contents: &[u8], exec: bool) {
        let p = self.under_root(&rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, contents).expect("write file");
        if exec {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&p).unwrap().permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&p, perms).unwrap();
            }
        }
    }

    pub fn run_cli<I, S>(&mut self, args: I) -> std::process::Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        use assert_cmd::prelude::*;
        use std::process::Command;
        let mut cmd = Command::cargo_bin("oxidizr-arch").expect("cargo bin oxidizr-arch");
        cmd.args(args);
        for (k, v) in &self.envs { cmd.env(k, v); }
        cmd.env("PAGER", "cat");
        cmd.output().expect("run oxidizr-arch")
    }
}
