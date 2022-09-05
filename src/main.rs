#![warn(clippy::all, rust_2018_idioms)]
// Do not hide console window on Windows for now
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use self_update::cargo_crate_version;

pub fn do_self_update() -> Result<(), Box<dyn ::std::error::Error>> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("GeckoEidechse")
        .repo_name("northstar_dev_testing_helper_tool")
        .bin_name("northstar_dev_testing_helper_tool_bin") // <-- name of the binary in the zip to use to replcae current version with
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .no_confirm(true)
        .build()?
        .update()?;
    println!("Update status: `{}`!", status.version());
    Ok(())
}

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    if do_self_update().is_err() {
        println!("Failed fetching update!");
    }

    let mut native_options = eframe::NativeOptions::default();

    // Run in fullscreen if user is Steam Deck (or just called `deck`)
    // in the future, this should check for more Steam Deck specific settings
    if whoami::username() == "deck" {
        native_options.maximized = true;
    }

    eframe::run_native(
        "northstar_dev_testing_helper_tool",
        native_options,
        Box::new(|cc| Box::new(northstar_dev_testing_helper_tool::TemplateApp::new(cc))),
    );
}
