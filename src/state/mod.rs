use crate::logging::{audit_event_fields, AuditFields};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const DEFAULT_STATE_DIR: &str = "/var/lib/oxidizr-arch";
const DEFAULT_LOG_DIR: &str = "/var/log";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    pub enabled_experiments: Vec<String>,
    pub managed_targets: Vec<String>,
    pub timestamp: String,
}

fn state_dir_or(default_override: Option<&Path>) -> PathBuf {
    default_override
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STATE_DIR))
}

pub fn load_state(state_dir: Option<&Path>) -> State {
    let dir = state_dir_or(state_dir);
    let path = dir.join("state.json");
    if let Ok(bytes) = fs::read(&path) {
        if let Ok(s) = serde_json::from_slice::<State>(&bytes) {
            return s;
        }
    }
    State::default()
}

pub fn save_state(state_dir: Option<&Path>, mut st: State, dry_run: bool) -> Result<()> {
    if dry_run {
        tracing::info!("[dry-run] skip save_state to {:?}", state_dir);
        return Ok(());
    }
    let dir = state_dir_or(state_dir);
    fs::create_dir_all(&dir)?;
    st.timestamp = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let path = dir.join("state.json");
    let body = serde_json::to_vec_pretty(&st).map_err(|e| Error::Other(e.to_string()))?;
    fs::write(&path, &body)?;
    let _ = audit_event_fields(
        "state",
        "save",
        "success",
        &AuditFields {
            artifacts: Some(vec![path.display().to_string()]),
            ..Default::default()
        },
    );
    Ok(())
}

pub fn write_state_report(
    state_dir: Option<&Path>,
    log_dir_override: Option<&Path>,
) -> Result<PathBuf> {
    let st = load_state(state_dir);
    let log_dir = log_dir_override
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LOG_DIR));
    let report = log_dir.join("oxidizr-arch/state-report.txt");
    if let Some(parent) = report.parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&report)?;
    writeln!(f, "State at {}", st.timestamp).ok();
    for t in &st.managed_targets {
        let path = Path::new(t);
        let status = match fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_symlink() => {
                if let Ok(target) = fs::read_link(path) {
                    format!("symlink -> {}", target.display())
                } else {
                    "symlink -> <unreadable>".to_string()
                }
            }
            Ok(_) => "regular".to_string(),
            Err(_) => "missing".to_string(),
        };
        writeln!(f, "{}\t{}", t, status).ok();
    }
    let _ = audit_event_fields(
        "state",
        "report",
        "success",
        &AuditFields {
            artifacts: Some(vec![report.display().to_string()]),
            ..Default::default()
        },
    );
    Ok(report)
}

pub fn set_enabled(
    state_dir: Option<&Path>,
    dry_run: bool,
    name: &str,
    enabled: bool,
    managed: &[PathBuf],
) -> Result<State> {
    let mut st = load_state(state_dir);
    if enabled {
        if !st.enabled_experiments.iter().any(|n| n == name) {
            st.enabled_experiments.push(name.to_string());
        }
        // merge managed targets
        for p in managed {
            let s = p.display().to_string();
            if !st.managed_targets.iter().any(|x| x == &s) {
                st.managed_targets.push(s);
            }
        }
    } else {
        st.enabled_experiments.retain(|n| n != name);
        // remove managed targets for this experiment
        // (callers pass the list of targets to remove)
        for p in managed {
            let s = p.display().to_string();
            st.managed_targets.retain(|x| x != &s);
        }
    }
    save_state(state_dir, st.clone(), dry_run)?;
    Ok(st)
}
