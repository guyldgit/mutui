mod app;
mod backend;
mod config;
mod input;
mod library;
mod picker;
mod run;
mod ui;

pub fn run(handle: mutui_core::Handle) -> mutui_core::Result<()> {
    run::run(handle)
}
