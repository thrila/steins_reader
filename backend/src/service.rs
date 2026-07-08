use std::{
    ffi::OsString,
    net::SocketAddr,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use anyhow::{Context, Result};
use axum::Router;
use tokio::{runtime::Builder, sync::oneshot};
use tracing::info;
use tracing_appender::rolling;
use tracing_log::LogTracer;
use tracing_subscriber::EnvFilter;

use crate::{
    app,
    config::{AppConfig, RuntimeMode},
    db,
};

const SERVICE_NAME: &str = "SteinsReader";

#[cfg(windows)]
define_windows_service!(ffi_service_main, service_main);

pub fn run() -> Result<()> {
    #[cfg(windows)]
    {
        if let Err(err) = windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        {
            eprintln!("SCM start failed, falling back to console mode: {err}");
            return run_console(std::env::args_os().collect());
        }

        Ok(())
    }

    #[cfg(not(windows))]
    {
        run_console(std::env::args_os().collect())
    }
}

#[cfg(windows)]
fn service_main(_arguments: Vec<OsString>) {
    if let Err(err) = run_service() {
        tracing::error!(error = %err, "Windows service exited with an error");
    }
}

fn run_console(arguments: Vec<OsString>) -> Result<()> {
    let config = AppConfig::load(RuntimeMode::Console, &arguments)?;
    let _log_guard = init_logging(&config)?;
    let runtime = build_runtime()?;

    runtime.block_on(async move {
        let (listener, app) = prepare_server(&config).await?;
        let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
        info!(
            %addr,
            db_path = %config.db_path.display(),
            frontend_dir = %config.frontend_dir.display(),
            "Steins Reader listening in console mode"
        );

        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
            .context("HTTP server failed")
    })
}

#[cfg(windows)]
fn run_service() -> Result<()> {
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};

    let arguments = std::env::args_os().collect::<Vec<_>>();
    let config = AppConfig::load(RuntimeMode::Service, &arguments)?;
    let _log_guard = init_logging(&config)?;
    let runtime = build_runtime()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let shutdown_tx = Arc::new(Mutex::new(Some(shutdown_tx)));
    let status_handle_slot: Arc<
        OnceLock<windows_service::service_control_handler::ServiceStatusHandle>,
    > = Arc::new(OnceLock::new());

    let handler_status_slot = Arc::clone(&status_handle_slot);
    let handler_shutdown_tx = Arc::clone(&shutdown_tx);
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                if let Some(status_handle) = handler_status_slot.get() {
                    let _ = status_handle.set_service_status(service_status(
                        ServiceState::StopPending,
                        ServiceControlAccept::empty(),
                        1,
                        Duration::from_secs(5),
                        ServiceExitCode::Win32(0),
                    ));
                }

                if let Some(sender) = handler_shutdown_tx
                    .lock()
                    .ok()
                    .and_then(|mut slot| slot.take())
                {
                    let _ = sender.send(());
                }

                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;
    let _ = status_handle_slot.set(status_handle.clone());
    status_handle.set_service_status(service_status(
        ServiceState::StartPending,
        ServiceControlAccept::empty(),
        1,
        Duration::from_secs(5),
        ServiceExitCode::Win32(0),
    ))?;

    let running_status_handle = status_handle.clone();
    let result = runtime.block_on(async move {
        let (listener, app) = prepare_server(&config).await?;
        let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
        running_status_handle.set_service_status(service_status(
            ServiceState::Running,
            ServiceControlAccept::STOP,
            0,
            Duration::default(),
            ServiceExitCode::Win32(0),
        ))?;

        info!(
            %addr,
            db_path = %config.db_path.display(),
            frontend_dir = %config.frontend_dir.display(),
            "Steins Reader service is running"
        );

        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .context("HTTP server failed")
    });

    let stopped_status = if result.is_ok() {
        service_status(
            ServiceState::Stopped,
            ServiceControlAccept::empty(),
            0,
            Duration::default(),
            ServiceExitCode::Win32(0),
        )
    } else {
        service_status(
            ServiceState::Stopped,
            ServiceControlAccept::empty(),
            0,
            Duration::default(),
            ServiceExitCode::Win32(1),
        )
    };
    let _ = status_handle.set_service_status(stopped_status);

    result
}

fn build_runtime() -> Result<tokio::runtime::Runtime> {
    Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build Tokio runtime")
}

fn init_logging(config: &AppConfig) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    std::fs::create_dir_all(&config.log_dir).with_context(|| {
        format!(
            "failed to create log directory {}",
            config.log_dir.display()
        )
    })?;

    LogTracer::init().context("failed to install log tracer")?;

    let file_appender = rolling::daily(&config.log_dir, "steins-reader.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("failed to install tracing subscriber")?;

    Ok(guard)
}

async fn prepare_server(config: &AppConfig) -> Result<(tokio::net::TcpListener, Router)> {
    let db = Arc::new(db::init(&config.db_path)?);
    let app = app::build_app(config, db);
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind {}", addr))?;

    Ok((listener, app))
}

#[cfg(windows)]
fn service_status(
    current_state: windows_service::service::ServiceState,
    controls_accepted: windows_service::service::ServiceControlAccept,
    checkpoint: u32,
    wait_hint: Duration,
    exit_code: windows_service::service::ServiceExitCode,
) -> windows_service::service::ServiceStatus {
    windows_service::service::ServiceStatus {
        service_type: windows_service::service::ServiceType::OWN_PROCESS,
        current_state,
        controls_accepted,
        exit_code,
        checkpoint,
        wait_hint,
        process_id: None,
    }
}
