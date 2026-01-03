#![allow(unused)]

use clap::Parser;
use log::{debug, error, info, warn};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(version, about="Rename images based on their folder.", long_about = None)]
struct Args {
    /// The source folder containing the images.
    source: PathBuf,

    /// The target folder to put the renamed images. Defaults to ".".
    #[arg(default_value = ".")]
    destination: PathBuf,

    /// Copy instead of moving.
    #[arg(short, long)]
    copy: bool,

    /// The image prefix to use. Defaults to the source folder name.
    #[arg(short, long)]
    prefix: Option<String>,

    /// Log file actions.
    #[arg(short, long)]
    verbose: bool,

    /// Do nothing.
    #[arg(short, long)]
    dry_run: bool,
}

/// Move or copy images from the source path to the destination path with a specified prefix.
///
/// # Arguments
/// * `source_path` - The path to the source directory.
/// * `destination_path` - The path to the destination directory.
/// * `copy_file` - A boolean indicating whether to copy (true) or move (false) the files.
/// * `prefix` - The prefix to be added to the destination file names.
/// * `verbose` - A boolean indicating whether to log file actions.
/// * `dry_run` - A boolean indicating whether to perform a dry run (no actual file operations).
///
/// # Returns
/// A Result indicating success or failure.
fn move_images(
    source_path: PathBuf,
    destination_path: PathBuf,
    copy_file: bool,
    prefix: &str,
    verbose: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Source path: {:?}", source_path);
    debug!("Destination path: {:?}", destination_path);
    debug!("Copy file: {}", copy_file);
    debug!("Prefix: {}", prefix);
    debug!("Verbose: {}", verbose);
    debug!("Dry run: {}", dry_run);

    let source_files: Vec<_> = get_source_files(source_path)?;

    let op = if dry_run {
        // Dry run: no operation
        |_: &PathBuf, _: &PathBuf| -> std::io::Result<()> { Ok(()) }
    } else if copy_file {
        |source: &PathBuf, destination: &PathBuf| -> std::io::Result<()> {
            fs::copy(source, destination).and(Ok(()))
        }
    } else {
        |source: &PathBuf, destination: &PathBuf| -> std::io::Result<()> {
            fs::rename(source, destination)
        }
    };

    let dry_run_prefix = if dry_run { "[dry-run] " } else { "" };
    let name = if copy_file { "copy" } else { "move" };

    for (source_file, destination_file) in
        generate_source_destination_pairs(source_files, destination_path, prefix)
    {
        let op_text = format!(
            "{}{} {:?} -> {:?}",
            dry_run_prefix, name, source_file, destination_file
        );
        if verbose {
        } else {
            debug!("{}", op_text);
        }

        match op(&source_file, &destination_file) {
            Ok(_) if verbose => println!("{}", op_text),
            Ok(_) => debug!("{}", op_text),
            Err(e) => error!(
                "Failed to {} {:?} -> {:?}: {}",
                name, source_file, destination_file, e
            ),
        }
    }

    Ok(())
}

/// Retrieve all source files from the specified source path.
///
/// # Arguments
/// * `source_path` - The path to the source directory.
///
/// # Returns
/// A vector of file paths contained in the source directory.
fn get_source_files(
    source_path: PathBuf,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let source_files: Vec<_> = fs::read_dir(source_path)?
        .filter_map(|f| match f {
            Ok(entry) => match entry.file_type() {
                Ok(file_type) => {
                    if file_type.is_file() {
                        Some(entry.path())
                    } else {
                        debug!("Ignoring non-file entry: {:?}", entry.path());
                        None
                    }
                }
                Err(err) => {
                    warn!(
                        "Failed to get file type for entry {:?}: {}",
                        entry.path(),
                        err
                    );
                    None
                }
            },
            Err(err) => {
                warn!("Error reading source directory entry: {}", err);
                None
            }
        })
        .collect();

    Ok(source_files)
}

/// Generate source and destination file path pairs.
///
/// This function takes a list of source file paths, a destination directory path,
/// and a prefix string. It generates destination file paths by appending the prefix
/// and an index to the original file name, preserving the original file extension.
///
/// # Arguments
/// * `source_files` - A vector of source file paths.
/// * `destination_path` - The destination directory path.
/// * `prefix` - The prefix to be added to the destination file names as a string slice.
///
/// # Returns
/// A vector of tuples, each containing a source file path and the corresponding destination file path.
fn generate_source_destination_pairs(
    source_files: Vec<PathBuf>,
    destination_path: PathBuf,
    prefix: &str,
) -> Vec<(PathBuf, PathBuf)> {
    source_files
        .into_iter()
        .enumerate()
        .map(|(index, source_file)| {
            let destination_file = destination_path.join(format!(
                "{}_{}{}",
                prefix,
                index,
                source_file
                    .extension()
                    .map_or(String::new(), |ext| format!(".{}", ext.to_string_lossy()))
            ));
            (source_file, destination_file)
        })
        .collect()
}

/// Get the source folder name from the provided source path or use the provided prefix.
///
/// If a prefix is provided in the arguments, it is returned. Otherwise, the function extracts the folder name from
/// the source path.
///
/// # Arguments
/// * `args` - A reference to the command-line arguments.
///
/// # Returns
/// A `Result` containing the source folder name or an error message.
fn get_prefix(args: &Args) -> Result<String, String> {
    match &args.prefix {
        Some(p) => Ok(p.clone()),
        None => Ok((&args.source)
        .file_name()
        .ok_or(
            "Cannot determine prefix from source path. Supply a prefix using the --prefix option.",
        )?
        .to_string_lossy()
        .to_string()),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();
    move_images(
        args.source
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize source path: {}", e))?,
        args.destination
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize destination path: {}", e))?,
        args.copy,
        &get_prefix(&args)?,
        args.verbose,
        args.dry_run,
    )
    .map_err(|e| format!("Error moving images: {}", e))?;
    Ok(())
}
