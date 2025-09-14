use std::fmt as StdFmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Once,
};

use tracing::{Event, Level};
use tracing_log::LogTracer;
use tracing_subscriber::filter::{FilterFn, LevelFilter};
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields, Writer};
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;
use tracing_subscriber::{layer::SubscriberExt, Registry};

use super::audit::AUDIT_LOG_PATH;

static INIT: Once = Once::new();
static ANNOUNCED_SINK: AtomicBool = AtomicBool::new(false);

/// Initialize global tracing subscribers for human logs and audit JSONL.
///
/// - Human logs go to stderr with level controlled by VERBOSE env (0..3)
/// - Audit events (target="audit") are written as single-line JSON to AUDIT_LOG_PATH
pub fn init_logging() {
    INIT.call_once(|| {
        // Capture legacy log:: macros and route them into tracing
        let _ = LogTracer::init();

        // Map VERBOSE to a LevelFilter; fallback to INFO when unset
        let level = match std::env::var("VERBOSE")
            .ok()
            .and_then(|s| s.parse::<u8>().ok())
        {
            Some(0) => LevelFilter::ERROR,
            Some(1) => LevelFilter::INFO,
            Some(2) => LevelFilter::DEBUG,
            Some(3) => LevelFilter::TRACE,
            _ => LevelFilter::INFO,
        };

        // Human-readable layer to stderr with custom prefix per VERBOSITY.md
        let distro = read_distro_id();
        let human_layer = fmt::layer()
            .event_format(HumanFormatter { distro })
            .with_writer(io::stderr)
            .with_ansi(atty::is(atty::Stream::Stderr))
            .with_filter(level);

        // Decide whether to attach audit sink (disabled in dry-run)
        let dry_run = std::env::var("OXIDIZR_DRY_RUN").ok().as_deref() == Some("1");
        if dry_run {
            tracing::info!("audit sink: disabled (dry-run)");
        }

        // JSONL audit layer to file, only for target=="audit". We provide our own timestamp field
        // inside audit_event, so we do not attach a timer here to avoid duplicate timestamps.
        let audit_layer = fmt::layer()
            .json()
            .flatten_event(true) // move fields to top-level
            .with_current_span(false)
            .with_span_list(false)
            .with_level(false)
            .with_target(false)
            .with_writer(AuditMakeWriter::new(PathBuf::from(AUDIT_LOG_PATH)))
            .with_filter(FilterFn::new(move |meta| {
                !dry_run && meta.target() == "audit"
            }));
        let subscriber = Registry::default().with(human_layer).with(audit_layer);

        // Install the composed subscriber
        let _ = subscriber.try_init();
    });
}

/// A MakeWriter that appends to an audit log file.
/// Attempts primary path and falls back to $HOME/.oxidizr-arch-audit.log on error.
struct AuditMakeWriter {
    primary: PathBuf,
}

impl AuditMakeWriter {
    pub fn new(primary: PathBuf) -> Self {
        Self { primary }
    }

    fn open(&self) -> io::Result<std::fs::File> {
        // Try primary path, fallback to HOME if needed
        match open_append(&self.primary) {
            Ok(f) => {
                if !ANNOUNCED_SINK.swap(true, Ordering::SeqCst) {
                    tracing::info!("audit sink: {}", self.primary.display());
                }
                Ok(f)
            }
            Err(_) => {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                let fallback = Path::new(&home).join(".oxidizr-arch-audit.log");
                if !ANNOUNCED_SINK.swap(true, Ordering::SeqCst) {
                    tracing::info!("audit sink: {} (fallback)", fallback.display());
                }
                open_append(&fallback)
            }
        }
    }
}

fn open_append(path: &Path) -> io::Result<std::fs::File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for AuditMakeWriter {
    type Writer = AuditWriter;
    fn make_writer(&'a self) -> Self::Writer {
        let file = self.open().expect("failed to open audit log file");
        AuditWriter { file: Some(file) }
    }
}

/// Simple wrapper that implements Write over a std::fs::File
pub struct AuditWriter {
    file: Option<std::fs::File>,
}

impl io::Write for AuditWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(f) = self.file.as_mut() {
            f.write(buf)
        } else {
            Ok(buf.len())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        if let Some(f) = self.file.as_mut() {
            f.flush()
        } else {
            Ok(())
        }
    }
}

/// Human-readable formatter: prints "[<distro>][v<level>] message"
struct HumanFormatter {
    distro: String,
}

impl<S, N> FormatEvent<S, N> for HumanFormatter
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> StdFmt::Result {
        let lvl = *event.metadata().level();
        let vnum = match lvl {
            Level::ERROR => 0,
            Level::WARN => 1,
            Level::INFO => 1,
            Level::DEBUG => 2,
            Level::TRACE => 3,
        };
        // Extract the message (and any other fields) from the event
        use tracing::field::{Field, Visit};
        struct MsgVisitor {
            msg: Option<String>,
            rest: Vec<String>,
        }
        impl Visit for MsgVisitor {
            fn record_debug(&mut self, field: &Field, value: &dyn StdFmt::Debug) {
                let name = field.name();
                if name == "message" {
                    self.msg = Some(format!("{:?}", value));
                } else {
                    self.rest.push(format!("{}={:?}", name, value));
                }
            }
        }
        let mut vis = MsgVisitor {
            msg: None,
            rest: Vec::new(),
        };
        event.record(&mut vis);
        let content = if let Some(m) = vis.msg {
            m
        } else {
            vis.rest.join(" ")
        };
        write!(writer, "[{}][v{}] {}\n", self.distro, vnum, content)
    }
}

fn read_distro_id() -> String {
    if let Ok(txt) = fs::read_to_string("/etc/os-release") {
        for line in txt.lines() {
            if let Some(rest) = line.strip_prefix("ID=") {
                return rest.trim_matches('"').to_ascii_lowercase();
            }
        }
    }
    "unknown".to_string()
}
