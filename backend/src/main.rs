#[cfg(windows)]
#[macro_use]
extern crate windows_service;

mod app;
mod config;
mod db;
mod handlers;
mod kalman;
mod models;
mod service;

fn main() -> anyhow::Result<()> {
    service::run()
}
