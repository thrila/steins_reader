use anyhow::{bail, ensure, Context, Result};
use serde::Deserialize;
use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeMode {
    Console,
    Service,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub db_path: PathBuf,
    pub frontend_dir: PathBuf,
    pub log_dir: PathBuf,
}

#[derive(Debug, Deserialize, Default)]
struct FileConfig {
    port: Option<u16>,
    db_path: Option<PathBuf>,
    frontend_dir: Option<PathBuf>,
    log_dir: Option<PathBuf>,
}

impl AppConfig {
    pub fn load(mode: RuntimeMode, args: &[OsString]) -> Result<Self> {
        let mut config = Self::defaults(mode);

        if let Some(config_path) =
            find_config_path(args).or_else(|| env::var_os("STEINS_CONFIG_PATH").map(PathBuf::from))
        {
            if mode == RuntimeMode::Service && !config_path.is_absolute() {
                bail!(
                    "STEINS_CONFIG_PATH or --config must be an absolute path when running as a Windows service"
                );
            }

            let file_config = read_file_config(&config_path)?;
            config.apply(file_config);
        }

        if let Some(port) = env::var("STEINS_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
        {
            config.port = port;
        }
        if let Some(db_path) = env::var_os("STEINS_DB_PATH") {
            config.db_path = PathBuf::from(db_path);
        }
        if let Some(frontend_dir) = env::var_os("STEINS_FRONTEND_DIR") {
            config.frontend_dir = PathBuf::from(frontend_dir);
        }
        if let Some(log_dir) = env::var_os("STEINS_LOG_DIR") {
            config.log_dir = PathBuf::from(log_dir);
        }

        if mode == RuntimeMode::Service {
            ensure!(
                !config.db_path.as_os_str().is_empty(),
                "service mode requires DB_PATH via config file or STEINS_DB_PATH"
            );
            ensure!(
                !config.frontend_dir.as_os_str().is_empty(),
                "service mode requires FRONTEND_DIR via config file or STEINS_FRONTEND_DIR"
            );
            ensure!(
                !config.log_dir.as_os_str().is_empty(),
                "service mode requires LOG_DIR via config file or STEINS_LOG_DIR"
            );
            ensure!(
                config.db_path.is_absolute(),
                "service DB path must be absolute"
            );
            ensure!(
                config.frontend_dir.is_absolute(),
                "service frontend path must be absolute"
            );
            ensure!(
                config.log_dir.is_absolute(),
                "service log path must be absolute"
            );
        }

        Ok(config)
    }

    fn defaults(mode: RuntimeMode) -> Self {
        match mode {
            RuntimeMode::Console => Self {
                port: 1337,
                db_path: PathBuf::from("steins_reader.db"),
                frontend_dir: PathBuf::from("dist"),
                log_dir: PathBuf::from("logs"),
            },
            RuntimeMode::Service => Self {
                port: 1337,
                db_path: PathBuf::new(),
                frontend_dir: PathBuf::new(),
                log_dir: PathBuf::new(),
            },
        }
    }

    fn apply(&mut self, file_config: FileConfig) {
        if let Some(port) = file_config.port {
            self.port = port;
        }
        if let Some(db_path) = file_config.db_path {
            self.db_path = db_path;
        }
        if let Some(frontend_dir) = file_config.frontend_dir {
            self.frontend_dir = frontend_dir;
        }
        if let Some(log_dir) = file_config.log_dir {
            self.log_dir = log_dir;
        }
    }
}

fn read_file_config(path: &Path) -> Result<FileConfig> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse JSON config file {}", path.display()))
}

fn find_config_path(args: &[OsString]) -> Option<PathBuf> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == OsStr::new("--config") {
            return iter.next().map(PathBuf::from);
        }

        let value = arg.to_string_lossy();
        if let Some(path) = value.strip_prefix("--config=") {
            return Some(PathBuf::from(path));
        }
    }
    None
}
