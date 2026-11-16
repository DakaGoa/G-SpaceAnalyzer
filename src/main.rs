// G-SpaceAnalyzer — a fast, private disk space analyzer for Windows.
//
// Entry point: either runs headless (when an export is requested) or opens the
// undecorated native window and hands control to the egui application in `app`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cleanup;
mod cli;
mod dupe_ui;
mod dupes;
mod export;
mod modal_ui;
mod model;
mod platform;
mod scan;
mod settings;
mod snapshots;
mod table_ui;
mod theme;
mod treemap;
mod util;
mod views;
mod views_extra;

use std::ffi::OsString;
use std::path::PathBuf;

use app::{App, Startup};
use eframe::egui;

fn main() {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();

    if cli::is_headless(&args) {
        std::process::exit(cli::run(&args));
    }

    if let Err(error) = run_gui(&args) {
        eprintln!("G-SpaceAnalyzer could not start: {error}");
        std::process::exit(1);
    }
}

fn run_gui(args: &[OsString]) -> eframe::Result<()> {
    let startup = parse_startup(args);

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("G-SpaceAnalyzer")
        .with_app_id("G-SpaceAnalyzer")
        .with_inner_size([1320.0, 860.0])
        .with_min_inner_size([960.0, 620.0])
        .with_decorations(false)
        .with_transparent(false);

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "G-SpaceAnalyzer",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, startup)))),
    )
}

/// Interpret the command line for the windowed app.
///
/// `G-SpaceAnalyzer [path] [--demo] [--scan <path>]`
fn parse_startup(args: &[OsString]) -> Startup {
    let mut demo = false;
    let mut scan: Option<PathBuf> = None;

    let mut index = 0;
    while index < args.len() {
        let text = args[index].to_string_lossy();
        match text.as_ref() {
            "--demo" | "-d" => demo = true,
            "--scan" | "-s" => {
                index += 1;
                if let Some(path) = args.get(index) {
                    scan = Some(PathBuf::from(path));
                }
            }
            "--help" | "-h" | "/?" => {
                print_help();
                std::process::exit(0);
            }
            other if other.starts_with('-') => {}
            _ => {
                let path = PathBuf::from(&args[index]);
                if path.exists() {
                    scan = Some(path);
                }
            }
        }
        index += 1;
    }

    Startup { demo, scan }
}

fn print_help() {
    println!(concat!(
        "G-SpaceAnalyzer ",
        env!("CARGO_PKG_VERSION"),
        " — free and open source disk space analyzer\n\
         \n\
         Usage: G-SpaceAnalyzer [path] [options]\n\
         \n\
         Arguments:\n\
           path               Folder or drive to scan on startup\n\
         \n\
         Options:\n\
           -d, --demo         Open the built-in sample volume\n\
           -s, --scan PATH    Folder or drive to scan\n\
           -e, --export FILE  Write a report and exit (headless)\n\
               --folders      Export the largest folders instead of files\n\
               --json         Export the full scan as JSON\n\
           -q, --quiet        Only report errors\n\
           -h, --help         Show this help\n"
    ));
}

fn load_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/icons/icon_256.png");
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}
