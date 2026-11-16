// Headless mode: scan and export without opening a window.
//
//   G-SpaceAnalyzer --scan C:\ --export report.csv --quiet
//   G-SpaceAnalyzer C:\ --export report.json --json
//   G-SpaceAnalyzer --scan C:\ --export folders.csv --folders
//
// Exit codes: 0 success, 1 scan/export failure, 2 bad usage.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::export;
use crate::platform;
use crate::scan::{self, ScanProgress};
use crate::util::{format_bytes, format_count, format_duration};

const FILE_ROW_LIMIT: usize = 200_000;
const FOLDER_ROW_LIMIT: usize = 50_000;

#[derive(Default)]
struct Options {
    scan: Option<PathBuf>,
    export: Option<PathBuf>,
    json: bool,
    folders: bool,
    quiet: bool,
}

pub fn is_headless(args: &[OsString]) -> bool {
    args.iter().any(|arg| {
        let text = arg.to_string_lossy();
        matches!(
            text.as_ref(),
            "--headless" | "--export" | "--json" | "--folders"
        ) || text.starts_with("--export=")
    })
}

pub fn run(args: &[OsString]) -> i32 {
    attach_console();
    let options = match parse(args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("Try G-SpaceAnalyzer --help");
            return 2;
        }
    };

    let Some(root) = options.scan.clone() else {
        eprintln!("No folder to scan. Pass a path or --scan <path>.");
        return 2;
    };
    if !root.exists() {
        eprintln!("Path not found: {}", root.display());
        return 1;
    }

    let label = platform::label_for_path(&root);
    if !options.quiet {
        println!("Scanning {} ...", root.display());
    }

    let progress = Arc::new(ScanProgress::default());
    let cancel = Arc::new(AtomicBool::new(false));
    let tree = match scan::scan_path(root.clone(), label, progress, cancel) {
        Ok(tree) => tree,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    if let Some(path) = &options.export {
        let result = if options.json {
            export::write_json(&tree, path)
        } else if options.folders {
            export::write_folders_csv(&tree, FOLDER_ROW_LIMIT, path)
        } else {
            let ids: Vec<u32> = tree.file_ids.iter().copied().take(FILE_ROW_LIMIT).collect();
            export::write_largest_csv(&tree, &ids, path)
        };
        match result {
            Ok(rows) => {
                if !options.quiet {
                    println!(
                        "Exported {} rows to {}",
                        format_count(rows),
                        path.display()
                    );
                }
            }
            Err(error) => {
                eprintln!("Export failed: {error}");
                return 1;
            }
        }
    }

    if !options.quiet {
        println!(
            "{} · {} files · {} folders · scanned in {}",
            format_bytes(tree.total_size),
            format_count(tree.total_files),
            format_count(tree.total_dirs),
            format_duration(tree.elapsed)
        );
        if tree.hardlinked_bytes > 0 {
            println!(
                "{} of hardlinked content counted once",
                format_bytes(tree.hardlinked_bytes)
            );
        }
        if tree.errors > 0 {
            println!("{} items could not be read (try an elevated scan)", format_count(tree.errors));
        }
    }

    0
}

fn parse(args: &[OsString]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut index = 0;
    while index < args.len() {
        let text = args[index].to_string_lossy().into_owned();
        match text.as_str() {
            "--scan" | "-s" => {
                index += 1;
                let value = args.get(index).ok_or("--scan needs a path")?;
                options.scan = Some(PathBuf::from(value));
            }
            "--export" | "-e" => {
                index += 1;
                let value = args.get(index).ok_or("--export needs a file name")?;
                options.export = Some(PathBuf::from(value));
            }
            "--json" => options.json = true,
            "--folders" => options.folders = true,
            "--quiet" | "-q" => options.quiet = true,
            "--headless" | "--exit" => {}
            "--help" | "-h" | "/?" => {
                print_help();
                std::process::exit(0);
            }
            other if other.starts_with("--export=") => {
                options.export = Some(PathBuf::from(&other["--export=".len()..]));
            }
            other if other.starts_with('-') => {}
            _ => {
                let path = PathBuf::from(&args[index]);
                if path.exists() {
                    options.scan = Some(path);
                }
            }
        }
        index += 1;
    }
    Ok(options)
}

fn print_help() {
    println!(concat!(
        "G-SpaceAnalyzer ",
        env!("CARGO_PKG_VERSION"),
        " — disk space analyzer\n\
         \n\
         Usage: SpaceAnalyzer [path] [options]\n\
         \n\
         Arguments:\n\
           path               Folder or drive to scan on startup\n\
         \n\
         Options:\n\
           -d, --demo         Open the built-in sample volume\n\
           -s, --scan PATH    Folder or drive to scan\n\
           -e, --export FILE  Write a report and exit (implies headless)\n\
               --folders      Export the largest folders instead of files\n\
               --json         Export the full scan as JSON\n\
           -q, --quiet        Only report errors\n\
           -h, --help         Show this help\n"
    ));
}

/// A windowed release build has no console of its own; borrow the caller's.
#[cfg(windows)]
fn attach_console() {
    use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_console() {}
