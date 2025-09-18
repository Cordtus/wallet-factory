use anyhow::Result;
use bip39::Mnemonic;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde_json;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use wallet_generator::{Args, KeyType, Wallet, generate_wallets_batch};

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate count
    const MAX_WALLETS: usize = 1_000_000_000;
    if args.count > MAX_WALLETS {
        return Err(anyhow::anyhow!("Too many wallets requested. Maximum is {} billion", MAX_WALLETS / 1_000_000_000));
    }

    // Estimate memory usage
    let estimated_memory_mb = (args.count * 400) / (1024 * 1024);
    if estimated_memory_mb > 100_000 {
        println!("⚠️  Warning: Estimated memory usage: ~{}GB", estimated_memory_mb / 1024);
        println!("   Ensure you have sufficient RAM available.");

        // Give user a chance to abort
        println!("\nPress Enter to continue or Ctrl+C to abort...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
    }

    // Get mnemonic
    let mnemonic_str = if let Some(m) = args.mnemonic {
        m
    } else {
        println!("Enter your mnemonic phrase:");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    // Parse mnemonic and create seed
    println!("Parsing mnemonic and generating seed...");
    let mnemonic = Mnemonic::parse(&mnemonic_str)
        .map_err(|e| anyhow::anyhow!("Invalid mnemonic: {}", e))?;
    let seed = mnemonic.to_seed("");

    // Configure thread pool
    let num_threads = if args.threads > 0 {
        args.threads
    } else {
        num_cpus::get()
    };

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();

    println!("\n⚡ Cosmos Wallet Generator");
    println!("Key type: {:?}", args.key_type);
    println!("Platform: {} {}", std::env::consts::OS, std::env::consts::ARCH);
    println!("Threads: {}", num_threads);
    println!("Memory: High-performance mode (using available RAM)");

    if matches!(args.key_type, KeyType::Ethsecp256k1) {
        println!("Note: Using ethsecp256k1 (Keccak256 hashing)");
    } else {
        println!("Note: Using standard secp256k1 (SHA256+RIPEMD160 hashing)");
    }

    println!("\nGenerating {} wallets...",
        args.count.to_string()
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect::<Vec<_>>()
            .join(","));

    let start_time = Instant::now();

    // Setup progress bar
    let pb = ProgressBar::new(args.count as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) | {msg}")?
            .progress_chars("#>-"),
    );

    // Progress counter
    let progress = Arc::new(AtomicUsize::new(0));
    let progress_clone = progress.clone();

    // Spawn progress updater thread
    let pb_clone = pb.clone();
    let progress_handle = std::thread::spawn(move || {
        let mut last_count = 0;
        let mut last_time = Instant::now();

        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));

            let current_count = progress_clone.load(Ordering::Relaxed);
            pb_clone.set_position(current_count as u64);

            let now = Instant::now();
            let time_diff = now.duration_since(last_time).as_secs_f64();

            if time_diff > 0.5 {
                let rate = (current_count - last_count) as f64 / time_diff;
                pb_clone.set_message(format!("{:.0} wallets/sec", rate));
                last_count = current_count;
                last_time = now;
            }

            if current_count >= args.count {
                break;
            }
        }
    });

    // Calculate optimal batch size based on thread count
    let wallets_per_thread = (args.count + num_threads - 1) / num_threads;

    println!("Generating wallets using {} threads ({} wallets per thread)...",
             num_threads, wallets_per_thread);

    // Generate all wallets in parallel
    let all_wallets: Vec<Wallet> = (0..num_threads)
        .into_par_iter()
        .flat_map(|thread_id| {
            let start_idx = thread_id * wallets_per_thread;
            let count = if thread_id == num_threads - 1 {
                args.count.saturating_sub(start_idx)
            } else {
                wallets_per_thread.min(args.count.saturating_sub(start_idx))
            };

            if count == 0 {
                Vec::new()
            } else {
                generate_wallets_batch(&seed, start_idx, count, &args.prefix, &args.key_type, progress.clone())
            }
        })
        .collect();

    // Wait for progress thread
    progress_handle.join().unwrap();
    pb.finish_with_message("Generation complete!");

    let generation_time = start_time.elapsed();

    println!("\nWriting {} wallets to file...", all_wallets.len());
    let write_start = Instant::now();

    // Create output directory if needed
    if let Some(parent) = Path::new(&args.output).parent() {
        fs::create_dir_all(parent)?;
    }

    // Write all wallets to file at once with large buffer
    let file = File::create(&args.output)?;
    let mut writer = BufWriter::with_capacity(64 * 1024 * 1024, file);

    // Write JSON array
    writer.write_all(b"[")?;
    for (i, wallet) in all_wallets.iter().enumerate() {
        if i > 0 {
            writer.write_all(b",")?;
        }
        writer.write_all(b"\n  ")?;
        serde_json::to_writer(&mut writer, wallet)?;
    }
    writer.write_all(b"\n]")?;
    writer.flush()?;

    let write_time = write_start.elapsed();
    let total_time = start_time.elapsed();

    // Get file size
    let file_size = fs::metadata(&args.output)?.len();
    let file_size_mb = file_size as f64 / (1024.0 * 1024.0);

    println!("\n✅ Performance Report:");
    println!("────────────────────");
    println!("📊 Wallets generated: {}",
        args.count.to_string()
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect::<Vec<_>>()
            .join(","));
    println!("⏱️  Generation time: {:.2}s", generation_time.as_secs_f64());
    println!("⏱️  Write time: {:.2}s", write_time.as_secs_f64());
    println!("⏱️  Total time: {:.2}s", total_time.as_secs_f64());
    println!("🚀 Generation rate: {:.0} wallets/sec", args.count as f64 / generation_time.as_secs_f64());
    println!("💾 File size: {:.2} MB", file_size_mb);
    println!("📁 Output: {}", args.output);

    Ok(())
}