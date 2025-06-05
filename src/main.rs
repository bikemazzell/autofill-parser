use autofill_parser::{
    models::{RawRecord, UserOutput},
    parser::{extract_emails, parse_line},
    processor::{choose_identifier, merge_records},
};
use clap::Parser;
use glob::glob;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use serde_json;
use sysinfo::{System, Pid, ProcessesToUpdate};
use rayon::prelude::*;

lazy_static::lazy_static! {
    static ref LOG_FILE: Mutex<File> = Mutex::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open("processing_errors.log")
            .expect("Failed to open log file")
    );
    static ref VERBOSE_MODE: Mutex<bool> = Mutex::new(false);
}

fn log_message(message: &str) {
    if let Ok(mut log_file) = LOG_FILE.lock() {
        if writeln!(log_file, "{}", message).is_err() {
            eprintln!("CRITICAL: Failed to write to log file: {}", message);
        }
    }

    if let Ok(verbose) = VERBOSE_MODE.lock() {
        if *verbose {
            eprintln!("{}", message);
        }
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser, value_name = "INPUT_DIR")]
    input: String,

    /// Path to the output file or folder.
    /// If a folder is specified, output will be saved as result.ndjson in that folder.
    #[clap(short, long, value_parser, value_name = "OUTPUT_PATH")]
    output: String,

    /// Activate verbose mode to print detailed processing information to the console.
    #[clap(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let config: autofill_parser::models::AppConfig = {
        let config_str = std::fs::read_to_string("config.json")?;
        serde_json::from_str(&config_str)?
    };

    if let Ok(mut verbose_mode) = VERBOSE_MODE.lock() {
        *verbose_mode = args.verbose;
    }

    // Clear or create the log file at the start of each run
    OpenOptions::new().create(true).write(true).truncate(true).open("processing_errors.log")?;
    log_message(&format!("Verbose mode: {}", args.verbose));

    let input_path = Path::new(&args.input);
    if !input_path.is_dir() {
        let err_msg = format!("Input path is not a directory: {}", args.input);
        log_message(&err_msg);
        return Err(err_msg.into());
    }

    let input_file_pattern = format!("{}/*", args.input.trim_end_matches('/'));

    let mut output_file_path = PathBuf::from(&args.output);
    if output_file_path.is_dir() {
        output_file_path.push("result.ndjson");
    } else {
        if let Some(parent_dir) = output_file_path.parent() {
            if !parent_dir.exists() {
                fs::create_dir_all(parent_dir)?;
            }
        }
    }

    let glob_results: Vec<_> = glob(&input_file_pattern)?.filter_map(Result::ok).collect();
    let files_processed_count = glob_results.len();

    let user_maps: Vec<HashMap<String, UserOutput>> = glob_results.par_iter().map(|path| {
        let mut local_users: HashMap<String, UserOutput> = HashMap::new();
        if path.is_file() {
            if args.verbose {
                println!("Processing file: {:?}", path.display());
            }
            let file = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    log_message(&format!("Error opening file {:?}: {}", path.display(), e));
                    return local_users;
                }
            };
            let reader = BufReader::new(file);
            for (line_number, line_result) in reader.lines().enumerate() {
                let line = match line_result {
                    Ok(l) => l,
                    Err(e) => {
                        log_message(&format!("Error reading line {} from file {:?}: {}", line_number + 1, path.display(), e));
                        continue;
                    }
                };
                if line.trim().is_empty() {
                    continue;
                }
                let parsed_data: RawRecord = parse_line(&line);
                if parsed_data.is_empty() {
                    log_message(&format!("Skipping empty parsed data from line {} in {:?}: '{}'", line_number + 1, path.display(), line));
                    continue;
                }
                let emails: Vec<String> = extract_emails(&parsed_data);
                if let Some(pk) = choose_identifier(&parsed_data, &emails) {
                    let mut current_record_other_fields = parsed_data.clone();
                    current_record_other_fields.remove("identifier");
                    current_record_other_fields.remove("emails");
                    local_users.entry(pk.clone())
                        .and_modify(|existing_user| {
                            merge_records(existing_user, &parsed_data);
                        })
                        .or_insert_with(|| UserOutput {
                            identifier: pk,
                            emails,
                            other_fields: current_record_other_fields,
                        });
                } else {
                    log_message(&format!("Skipping line due to no identifiable primary key (line {} in {:?}): '{}'", line_number + 1, path.display(), line));
                }
            }
        }
        local_users
    }).collect();

    let mut all_users: HashMap<String, UserOutput> = HashMap::new();
    for user_map in user_maps {
        for (k, v) in user_map {
            all_users.entry(k.clone())
                .and_modify(|existing_user| {
                    merge_records(existing_user, &v.other_fields);
                })
                .or_insert(v);
        }
        // Memory check and temp file swap can be done here if needed (not parallel section)
    }

    if files_processed_count == 0 {
        let no_files_msg = format!("No files found in the input directory to process: {}", args.input);
        log_message(&no_files_msg);
    } else {
        println!(
            "Writing {} processed users to {}",
            all_users.len(),
            output_file_path.display()
        );
        let mut out_file = File::create(&output_file_path)?;
        for user_record in all_users.values() {
            let json_string = serde_json::to_string(user_record)?;
            writeln!(out_file, "{}", json_string)?;
        }
    }
    println!("Processing complete. Check processing_errors.log for any issues.");
    Ok(())
} 