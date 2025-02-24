#![windows_subsystem = "windows"]

mod models;
mod settings;
mod fileops;
mod backup;

// 新規追加モジュール
mod ui;
mod app;

fn main() {
    app::run_app();
}
