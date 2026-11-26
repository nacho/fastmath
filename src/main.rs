/// The GLib log domain of the messages logged with `glib::debug!` and
/// `glib::warn!`.
pub const G_LOG_DOMAIN: &str = "fastmath";

mod config {
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/config.rs"));
}

mod application;
mod comparison;
mod game_view;
mod generator;
mod labels;
mod leaderboard;
mod level;
mod operation;
mod results_view;
mod round;
mod sound_player;
mod start_view;
#[cfg(test)]
mod ui_tests;
mod window;

use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, setlocale, textdomain};
use gio::prelude::*;
use std::path::PathBuf;

use application::FastmathApplication;

fn setup_i18n() -> Result<(), std::io::Error> {
    // SAFETY: Called at program start before any threads are spawned.
    unsafe { setlocale(LocaleCategory::LcAll, "") };

    let localedir = PathBuf::from(config::DATADIR).join("locale");
    bindtextdomain(config::PACKAGE, localedir)?;
    bind_textdomain_codeset(config::PACKAGE, "UTF-8")?;
    textdomain(config::PACKAGE)?;
    Ok(())
}

fn main() -> glib::ExitCode {
    static GLIB_LOGGER: glib::GlibLogger = glib::GlibLogger::new(
        glib::GlibLoggerFormat::Plain,
        glib::GlibLoggerDomain::CrateTarget,
    );
    let _ = glib::rs_log::set_logger(&GLIB_LOGGER);
    glib::rs_log::set_max_level(glib::rs_log::LevelFilter::Debug);

    if let Err(e) = setup_i18n() {
        glib::warn!("Could not set up translations: {e}");
    }

    if let Err(e) = gio::resources_register_include!("resources.gresource") {
        glib::warn!("Could not register the resources: {e}");
        return glib::ExitCode::FAILURE;
    }

    FastmathApplication::new().run()
}
