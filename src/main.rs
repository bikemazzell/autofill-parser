use autofill_parser::{
    models::UserOutput,
    constants::{
        BUFFER_SIZE_ULTRA, CHANNEL_BUFFER, BYTES_TO_KB, BYTES_TO_GB, PERCENT_DIVISOR,
        EMAIL_PARTS_COUNT
    },
};
use clap::Parser;
use glob::glob;
use rayon::prelude::*;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use serde_json;
use std::time::Instant;
use sysinfo::{System, Pid};


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser, value_name = "INPUT_DIR")]
    input: String,

    #[clap(short, long, value_parser, value_name = "OUTPUT_PATH")]
    output: String,

    #[clap(short, long)]
    verbose: bool,

    #[clap(short, long, default_value = "0")]
    threads: usize,
}

enum WorkerMessage {
    UserData(String, UserOutput),
}

#[derive(Clone)]
struct MemoryTracker {
    current_usage: Arc<Mutex<u64>>,
    available_budget: u64,
}

impl MemoryTracker {
    fn new(budget: u64) -> Self {
        Self {
            current_usage: Arc::new(Mutex::new(0)),
            available_budget: budget,
        }
    }
    
    fn can_allocate(&self, bytes: u64) -> bool {
        if let Ok(current) = self.current_usage.lock() {
            *current + bytes <= self.available_budget
        } else {
            false
        }
    }
    
    fn allocate(&self, bytes: u64) -> bool {
        if let Ok(mut current) = self.current_usage.lock() {
            if *current + bytes <= self.available_budget {
                *current += bytes;
                return true;
            }
        }
        false
    }
    
    fn try_allocate_with_retry(&self, bytes: u64, max_retries: u8) -> bool {
        for _ in 0..max_retries {
            if self.allocate(bytes) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        false
    }
    
    fn deallocate(&self, bytes: u64) {
        if let Ok(mut current) = self.current_usage.lock() {
            *current = current.saturating_sub(bytes);
        }
    }
    
    fn get_usage(&self) -> (u64, f64) {
        if let Ok(current) = self.current_usage.lock() {
            let percent = (*current as f64 / self.available_budget as f64) * 100.0;
            (*current, percent)
        } else {
            (0, 0.0)
        }
    }
}

fn estimate_file_memory_usage(file_path: &Path) -> Result<u64, Box<dyn Error>> {
    let metadata = std::fs::metadata(file_path)?;
    let file_size = metadata.len();
    
    let overhead = file_size / 2;
    match file_size.checked_add(overhead) {
        Some(total) => Ok(total),
        None => {
            eprintln!("Warning: File {} too large for memory estimation, using maximum safe value", file_path.display());
            Ok(u64::MAX / 2)
        }
    }
}

fn cleanup_temp_files(temp_files: &[PathBuf], temp_dir: &Path, verbose: bool) {
    let mut cleanup_errors = 0;
    
    for temp_path in temp_files {
        if let Err(e) = fs::remove_file(temp_path) {
            eprintln!("Warning: Failed to remove temp file {}: {}", temp_path.display(), e);
            cleanup_errors += 1;
        }
    }
    
    if temp_files.is_empty() {
        if let Err(e) = fs::remove_dir(temp_dir) {
            if verbose {
                eprintln!("Note: Could not remove temp directory {} (may not be empty): {}", temp_dir.display(), e);
            }
        }
    } else {
        match fs::read_dir(temp_dir) {
            Ok(mut entries) => {
                if entries.next().is_none() {
                    if let Err(e) = fs::remove_dir(temp_dir) {
                        if verbose {
                            eprintln!("Note: Could not remove empty temp directory {}: {}", temp_dir.display(), e);
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }
    
    if cleanup_errors > 0 {
        eprintln!("Warning: {} temp file cleanup errors occurred", cleanup_errors);
    }
}

fn parse_line_fast(line: &str) -> Option<(String, Vec<String>, HashMap<String, String>)> {
    if line.trim().is_empty() {
        return None;
    }

    let mut record = HashMap::new();
    let mut emails = Vec::new();
    let mut identifier = None;

    
    for pair in line.split(',') {
        if let Some(colon_pos) = pair.find(':') {
            if colon_pos < pair.len() {
                let key = pair[..colon_pos].trim();
                let value = if colon_pos + 1 < pair.len() {
                    pair[colon_pos + 1..].trim()
                } else {
                    ""
                };
                
                if !key.is_empty() && !value.is_empty() {
                    if value.contains('@') {
                        let parts: Vec<&str> = value.split('@').collect();
                        if parts.len() == EMAIL_PARTS_COUNT {
                            if let Some(domain) = parts.get(1) {
                                if domain.contains('.') {
                                    emails.push(value.to_lowercase());
                                }
                            }
                        }
                    }
                    
                    if identifier.is_none() {
                        match key {
                            "identifier" | "email" | "username" | "login" => {
                                identifier = Some(value.to_lowercase());
                            }
                            _ => {}
                        }
                    }
                    
                    record.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    if let Some(id) = identifier {
        Some((id, emails, record))
    } else if let Some(first_email) = emails.first() {
        Some((first_email.clone(), emails, record))
    } else {
        if let Some(fallback_value) = record.values().find(|v| !v.trim().is_empty()) {
            Some((fallback_value.to_string(), emails, record))
        } else {
            None
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if args.threads > 0 {
        if let Err(e) = rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global()
        {
            eprintln!("Warning: Failed to configure thread pool with {} threads: {}. Using default.", args.threads, e);
            eprintln!("Falling back to default thread count: {}", rayon::current_num_threads());
        }
    }

    let config: autofill_parser::models::AppConfig = {
        let config_str = std::fs::read_to_string("config.json")?;
        let config: autofill_parser::models::AppConfig = serde_json::from_str(&config_str)?;
        
        if let Err(e) = config.validate() {
            return Err(format!("Invalid configuration in config.json: {}", e).into());
        }
        
        if args.verbose {
            println!("Configuration validated successfully");
        }
        
        config
    };

    let mut sys = System::new_all();
    sys.refresh_memory();
    let total_mem = sys.total_memory()
        .checked_mul(BYTES_TO_KB)
        .unwrap_or_else(|| {
            eprintln!("Warning: Memory calculation overflow, using safe default");
            1_073_741_824 // 1GB fallback
        });
    
    let max_mem_bytes = total_mem
        .checked_mul(config.memory_usage_percent as u64)
        .and_then(|result| result.checked_div(PERCENT_DIVISOR))
        .unwrap_or_else(|| {
            eprintln!("Warning: Memory percentage calculation overflow, using 50% of total");
            total_mem / 2
        });

    let input_path = Path::new(&args.input);
    if !input_path.is_dir() {
        return Err(format!("Input path is not a directory: {}", args.input).into());
    }

    let mut output_file_path = PathBuf::from(&args.output);
    if output_file_path.is_dir() {
        output_file_path.push("result.ndjson");
    }

    let temp_dir = Path::new(&config.temp_directory);
    fs::create_dir_all(temp_dir)?;

    let pattern = format!("{}/*", args.input.trim_end_matches('/'));
    let files: Vec<_> = glob(&pattern)?.filter_map(Result::ok).collect();
    let total_files = files.len();

    let total_file_size_bytes: u64 = files.iter()
        .filter_map(|path| std::fs::metadata(path).ok())
        .map(|metadata| metadata.len())
        .sum();
    let total_file_size_gb = total_file_size_bytes as f64 / BYTES_TO_GB;
    let available_memory_gb = sys.available_memory() as f64 / BYTES_TO_GB;
    let memory_budget_gb = available_memory_gb * (config.memory_usage_percent as f64 / 100.0);

    println!("Processing {} files with {} threads", 
        total_files, 
        rayon::current_num_threads()
    );
    
    let (chunk_multiplier, max_records_limit, memory_check_freq) = if total_file_size_gb < config.small_dataset_threshold_gb {
        (config.chunk_size_multiplier / 4, config.max_records_before_swap * 2, config.memory_check_interval_secs * 2)
    } else if total_file_size_gb > config.large_dataset_threshold_gb {
        (config.chunk_size_multiplier * 4, config.safety_records_limit, 1)
    } else {
        (config.chunk_size_multiplier, config.max_records_before_swap, config.memory_check_interval_secs)
    };
    
    if args.verbose {
        println!("Dataset analysis:");
        println!("  Total file size: {:.2} GB", total_file_size_gb);
        println!("  Available memory: {:.2} GB", available_memory_gb);
        println!("  Memory budget: {:.2} GB ({}%)", memory_budget_gb, config.memory_usage_percent);
        
        let strategy = if total_file_size_gb < config.small_dataset_threshold_gb {
            "Small dataset - optimized for speed"
        } else if total_file_size_gb > config.large_dataset_threshold_gb {
            "Large dataset - optimized for memory efficiency"
        } else {
            "Medium dataset - balanced approach"
        };
        println!("  Strategy: {}", strategy);
        println!("Adaptive settings:");
        println!("  Max records before swap: {}", max_records_limit);
        println!("  Memory check frequency: {} seconds", memory_check_freq);
    }
    
    if args.verbose {
        sys.refresh_all();
        let available_memory_bytes = sys.available_memory();
        let total_memory_bytes = sys.total_memory(); 
        let available_gb = available_memory_bytes as f64 / BYTES_TO_GB;
        let total_gb = total_memory_bytes as f64 / BYTES_TO_GB;
        eprintln!("STARTUP DEBUG: Available memory: {:.2} GB / {:.2} GB total", 
            available_gb, total_gb);
    }

    let start_time = Instant::now();
    
    let memory_tracker = MemoryTracker::new((memory_budget_gb * BYTES_TO_GB) as u64);
    
    if args.verbose {
        println!("Memory tracker initialized with {:.2}GB budget", memory_budget_gb);
    }

    let (tx, rx) = mpsc::sync_channel::<WorkerMessage>(CHANNEL_BUFFER);
    let consumer_handle = {
        let output_path = output_file_path.clone();
        let temp_dir = temp_dir.to_path_buf();
        let _max_mem = max_mem_bytes;
        let verbose = args.verbose;
        let config_clone = config.clone();
        let adaptive_max_records = max_records_limit;
        let adaptive_memory_check_freq = memory_check_freq;
        let memory_tracker_clone = memory_tracker.clone();
        
        thread::spawn(move || {
            let mut all_users: HashMap<String, UserOutput> = HashMap::with_capacity(config_clone.hashmap_initial_capacity);
            let mut temp_files: Vec<PathBuf> = Vec::new();
            let _current_temp_file: Option<BufWriter<File>> = None;
            let mut sys = System::new_all();
            let _pid = Pid::from(std::process::id() as usize);
            let mut last_mem_check = Instant::now();
            let mut total_processed = 0usize;

            loop {
                match rx.recv() {
                    Ok(WorkerMessage::UserData(key, user)) => {
                        all_users.entry(key)
                            .and_modify(|existing| {
                                for (k, v) in &user.other_fields {
                                    existing.other_fields.entry(k.clone()).or_insert_with(|| v.clone());
                                }
                            })
                            .or_insert(user);

                        total_processed += 1;

                        let should_check_memory = last_mem_check.elapsed().as_secs() >= adaptive_memory_check_freq;
                        let should_check_records = total_processed % config_clone.record_check_interval == 0;
                        let force_swap = all_users.len() >= adaptive_max_records;
                        let safety_swap = all_users.len() >= config_clone.safety_records_limit;
                        
                        if should_check_memory || should_check_records || force_swap || safety_swap {
                            
                            sys.refresh_all();
                            let available_memory_bytes = sys.available_memory();
                            let total_memory_bytes = sys.total_memory();
                            let available_gb = available_memory_bytes as f64 / BYTES_TO_GB;
                            let _total_gb = total_memory_bytes as f64 / BYTES_TO_GB;
                            let memory_pressure = available_gb < config_clone.memory_pressure_threshold_gb;
                            let emergency_abort = available_gb < config_clone.emergency_abort_threshold_gb;
                            
                            if emergency_abort {
                                eprintln!("🚨 EMERGENCY: Available memory critically low ({:.2}GB). Halting to prevent system crash.", available_gb);
                                std::process::exit(1);
                            }
                            
                            if verbose && should_check_memory {
                                let (tracker_usage, tracker_percent) = memory_tracker_clone.get_usage();
                                println!("[{}] Memory: {:.2}GB system free, {:.2}GB tracked ({:.1}%)",
                                    chrono::Local::now().format("%H:%M:%S"),
                                    available_gb,
                                    tracker_usage as f64 / BYTES_TO_GB,
                                    tracker_percent
                                );
                            }
                            
                            
                            let should_swap = memory_pressure || force_swap || safety_swap;
                            
                            if should_swap {
                                    let temp_path = temp_dir.join(format!("temp_{}.ndjson", temp_files.len()));
                                    match File::create(&temp_path) {
                                        Ok(file) => {
                                            let mut writer = BufWriter::with_capacity(BUFFER_SIZE_ULTRA, file);
                                            
                                            let mut swap_errors = 0;
                                            for (_, user_record) in all_users.drain() {
                                                match serde_json::to_string(&user_record) {
                                                    Ok(json) => {
                                                        if let Err(e) = writeln!(writer, "{}", json) {
                                                            eprintln!("Error writing record to temp file: {}", e);
                                                            swap_errors += 1;
                                                            if swap_errors > 10 {
                                                                eprintln!("Too many write errors, aborting swap");
                                                                break;
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        eprintln!("Error serializing user record: {}", e);
                                                        swap_errors += 1;
                                                    }
                                                }
                                            }
                                            
                                            if let Err(e) = writer.flush() {
                                                eprintln!("Error flushing temp file: {}", e);
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("Critical: Failed to create temp file {}: {}. Data may be lost!", temp_path.display(), e);
                                            continue;
                                        }
                                    }
                                    
                                    temp_files.push(temp_path);
                                    all_users = HashMap::with_capacity(config_clone.hashmap_initial_capacity);
                                    
                                    if verbose {
                                        let reason = if safety_swap { 
                                            format!("safety limit ({}k records)", config_clone.safety_records_limit / 1000)
                                        } else if force_swap { 
                                            format!("adaptive limit ({}k records)", adaptive_max_records / 1000)
                                        } else { 
                                            "memory pressure".to_string()
                                        };
                                        println!("[{}] Swapped to temp file #{} ({}), {} records, {:.2} GB available",
                                            chrono::Local::now().format("%H:%M:%S"),
                                            temp_files.len(),
                                            &reason,
                                            total_processed,
                                            available_gb
                                        );
                                    }
                            }
                            last_mem_check = Instant::now();
                        }
                    }
                    Err(_) => break,
                }
            }

            println!("Writing {} records to output...", total_processed);
            
            let out_file = match File::create(&output_path) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Critical: Failed to create output file {}: {}", output_path.display(), e);
                    return total_processed;
                }
            };
            let mut out_writer = BufWriter::with_capacity(BUFFER_SIZE_ULTRA, out_file);

            let mut output_errors = 0;
            for temp_path in &temp_files {
                match File::open(temp_path) {
                    Ok(temp_file) => {
                        let reader = std::io::BufReader::with_capacity(BUFFER_SIZE_ULTRA, temp_file);
                        for line_result in reader.lines() {
                            match line_result {
                                Ok(line) => {
                                    if let Err(e) = writeln!(out_writer, "{}", line) {
                                        eprintln!("Error writing temp file line to output: {}", e);
                                        output_errors += 1;
                                        if output_errors > 100 {
                                            eprintln!("Too many output errors, aborting");
                                            break;
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error reading line from temp file {}: {}", temp_path.display(), e);
                                    output_errors += 1;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error opening temp file {}: {}", temp_path.display(), e);
                    }
                }
            }

            for user_record in all_users.values() {
                match serde_json::to_string(user_record) {
                    Ok(json) => {
                        if let Err(e) = writeln!(out_writer, "{}", json) {
                            eprintln!("Error writing user record to output: {}", e);
                            output_errors += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error serializing user record for output: {}", e);
                        output_errors += 1;
                    }
                }
            }

            if let Err(e) = out_writer.flush() {
                eprintln!("Error flushing output file: {}", e);
            }

            cleanup_temp_files(&temp_files, &temp_dir, verbose);

            total_processed
        })
    };

    let chunk_size = std::cmp::max(1, total_files / (rayon::current_num_threads() * chunk_multiplier));
    
    if args.verbose {
        println!("  Chunk size: {} files per chunk", chunk_size);
    }
    
    let verbose = args.verbose;
    files.par_chunks(chunk_size).for_each_with((tx.clone(), memory_tracker.clone()), |(tx, tracker), chunk| {
        for path in chunk {
            if !path.is_file() {
                continue;
            }

            let _file_size = match std::fs::metadata(path) {
                Ok(metadata) => metadata.len(),
                Err(e) => {
                    eprintln!("Warning: Cannot read metadata for file {}: {}", path.display(), e);
                    continue;
                }
            };
            
            
            let estimated_memory = match estimate_file_memory_usage(path) {
                Ok(size) => size,
                Err(e) => {
                    eprintln!("Warning: Cannot estimate memory for file {}: {}", path.display(), e);
                    continue;
                }
            };
            
            if !tracker.can_allocate(estimated_memory) {
                for _attempt in 0..10 {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if tracker.can_allocate(estimated_memory) {
                        break;
                    }
                }
            }
            
            let allocated_memory;
            
            if tracker.try_allocate_with_retry(estimated_memory, 5) {
                allocated_memory = estimated_memory;
            } else {
                let reduced_memory = std::cmp::min(estimated_memory / 2, tracker.available_budget / 10);
                if tracker.try_allocate_with_retry(reduced_memory, 5) {
                    allocated_memory = reduced_memory;
                } else {
                    let minimal_memory = 1_048_576;
                    if tracker.allocate(minimal_memory) {
                        allocated_memory = minimal_memory;
                    } else {
                        eprintln!("Warning: Processing file {} without memory tracking due to extreme memory pressure", path.display());
                        allocated_memory = 0;
                    }
                }
            }

            let file = match File::open(path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Error: Failed to open file {}: {}", path.display(), e);
                    tracker.deallocate(estimated_memory);
                    continue;
                }
            };
            
            let reader = std::io::BufReader::with_capacity(BUFFER_SIZE_ULTRA, file);
            let mut lines_processed = 0;
            let mut lines_skipped = 0;
            let mut read_errors = 0;
            
            for (line_num, line_result) in reader.lines().enumerate() {
                match line_result {
                    Ok(line_content) => {
                        if let Some((id, emails, mut other_fields)) = parse_line_fast(&line_content) {
                            other_fields.remove("identifier");
                            other_fields.remove("emails");
                            let user = UserOutput {
                                identifier: id.clone(),
                                emails,
                                other_fields,
                            };
                            if let Err(e) = tx.send(WorkerMessage::UserData(id, user)) {
                                eprintln!("Error: Failed to send user data from {}, line {}: {}", 
                                    path.display(), line_num + 1, e);
                                break;
                            }
                            lines_processed += 1;
                        } else {
                            lines_skipped += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: Failed to read line {} from {}: {}", line_num + 1, path.display(), e);
                        read_errors += 1;
                        if read_errors > 100 {
                            eprintln!("Too many read errors in file {}, aborting", path.display());
                            break;
                        }
                    }
                }
            }
            
            if verbose && (lines_processed > 0 || lines_skipped > 10 || read_errors > 0) {
                println!("[{}] File {}: {} processed, {} skipped, {} errors",
                    chrono::Local::now().format("%H:%M:%S"),
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    lines_processed,
                    lines_skipped,
                    read_errors
                );
            }
            
            if allocated_memory > 0 {
                tracker.deallocate(allocated_memory);
            }
        }
    });

    drop(tx);
    
    let total_users = match consumer_handle.join() {
        Ok(users) => users,
        Err(e) => {
            eprintln!("Critical: Consumer thread panicked: {:?}", e);
            eprintln!("Processing may be incomplete. Check output file for partial results.");
            
            eprintln!("Attempting emergency cleanup of temp files...");
            cleanup_temp_files(&[], &temp_dir, args.verbose);
            
            0
        }
    };
    
    let elapsed = start_time.elapsed().as_secs_f64();
    println!("\nProcessing complete!");
    println!("Total time: {:.2}s", elapsed);
    println!("Files processed: {}", total_files);
    println!("Total unique records: {}", total_users);
    println!("Performance: {:.0} files/sec, {:.0} rec/sec",
        total_files as f64 / elapsed,
        total_users as f64 / elapsed
    );

    Ok(())
}