use std::fs::{ File };
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;
use tracing::metadata::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter,
    Registry,
    fmt,
};
use tracing_subscriber::filter::Directive;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::util::SubscriberInitExt;

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
static ENV_FILTER_RELOADABLE_HANDLE: Mutex<Option<Handle<EnvFilter, Registry>>> =
    Mutex::new(None);
static GLOBAL_LOG_LEVEL: Mutex<Option<String>> = Mutex::new(None);
static MAX_LEVEL: Mutex<Option<LevelFilter>> = Mutex::new(None);
const LOG_LEVEL: &str = "KB_VIBE_LOG_LEVEL";
const DEFAULT_FILTER: LevelFilter = LevelFilter::ERROR;


#[derive(Debug, Error)]
pub enum LogError {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    TracingReload(#[from] tracing_subscriber::reload::Error)
}

pub struct LogArgs<T: AsRef<Path>> {
    pub log_level: Option<String>,
    pub log_to_stdout: bool,
    pub log_file_path: Option<T>,
    pub delete_old_log_file: bool,
}

pub struct LogGuard {
    _file_guard: Option<WorkerGuard>,
    _stdout_guard: Option<WorkerGuard>,
}

pub fn init_logging<T: AsRef<Path>>(args: LogArgs<T>) -> Result<LogGuard, LogError> {
    let filter_layer = create_filter_layer();
    let (reloadable_filter_layer, reloadable_handle) = tracing_subscriber::reload::Layer::new(filter_layer);
    ENV_FILTER_RELOADABLE_HANDLE.lock().unwrap().replace(reloadable_handle);

    let (file_layer, file_guard) = match args.log_file_path {
        Some(log_file_path) => {
            let log_path = log_file_path.as_ref();

            if let Some(parent) = log_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            if args.delete_old_log_file {
                std::fs::remove_file(log_path).ok();
            } else if log_path.exists() && std::fs::metadata(log_path)?.len() > MAX_FILE_SIZE {
                std::fs::remove_file(log_path)?;
            }

            let file = if args.delete_old_log_file {
                File::create(log_path)?
            } else {
                File::options().append(true).create(true).open(log_path)?
            };

            // Set permission for user on unix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = file.metadata() {
                    let mut permissions = metadata.permissions();
                    permissions.set_mode(0o600);
                    file.set_permissions(permissions).ok();
                }
            }

            let (non_blocking, guard) = tracing_appender::non_blocking(file);
            let file_layer = fmt::layer().with_line_number(true).with_writer(non_blocking);

            (Some(file_layer), Some(guard))
        },
        None => (None, None),
    };

    let (stdout_layer, stdout_guard) = if args.log_to_stdout {
        let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
        let stdout_layer = fmt::layer().with_line_number(true).with_writer(non_blocking);
        (Some(stdout_layer), Some(guard))
    } else {
        (None, None)
    };

    if let Some(level) = args.log_level {
        set_log_level(level)?;
    }

    let subscriber = tracing_subscriber::registry()
        .with(reloadable_filter_layer)
        .with(file_layer)
        .with(stdout_layer);

    subscriber.init();

    Ok(LogGuard{
        _file_guard: file_guard,
        _stdout_guard: stdout_guard,
    })
}

fn create_filter_layer() -> EnvFilter {
    let directive = Directive::from(DEFAULT_FILTER);
    
    let log_level = GLOBAL_LOG_LEVEL.lock().unwrap()
        .clone()
        .or_else(|| std::env::var(LOG_LEVEL).ok());
    
    match log_level {
        Some(level) => EnvFilter::builder()
            .with_default_directive(directive)
            .parse_lossy(level),
        None => EnvFilter::default().add_directive(directive)
    }
}

fn set_log_level(level: String) -> Result<String, LogError> {
    println!("Setting log level to {level:?}");

    let old_level = get_log_level();
    *GLOBAL_LOG_LEVEL.lock().unwrap() = Some(level);
    
    let filter_layer = create_filter_layer();
    *MAX_LEVEL.lock().unwrap() = filter_layer.max_level_hint();
    
    ENV_FILTER_RELOADABLE_HANDLE.lock().unwrap()
        .as_ref()
        .expect("set_log_level must be called after logging is initialized!")
        .reload(filter_layer)?;
    
    Ok(old_level)
}

fn get_log_level() -> String {
    GLOBAL_LOG_LEVEL.lock().unwrap()
        .clone()
        .unwrap_or_else(|| std::env::var(LOG_LEVEL).unwrap_or_else(|_| DEFAULT_FILTER.to_string()))
}

pub fn get_max_log_level() -> LevelFilter {
    let max_level = *MAX_LEVEL.lock().unwrap();
    max_level.unwrap_or_else(|| {
        let filter_layer = create_filter_layer();
        *MAX_LEVEL.lock().unwrap() = filter_layer.max_level_hint();
        filter_layer.max_level_hint().unwrap_or(DEFAULT_FILTER)
    })
}