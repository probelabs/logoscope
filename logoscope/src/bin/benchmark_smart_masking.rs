use std::fs;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <log_file>", args[0]);
        std::process::exit(1);
    }

    let content = fs::read_to_string(&args[1])?;
    let lines: Vec<&str> = content.lines().take(10000).collect(); // First 10k lines for benchmark

    println!("Benchmarking smart masking performance on {} lines...", lines.len());
    
    // Warmup
    for line in lines.iter().take(100) {
        let _ = logoscope::smart_masking::smart_mask_line(line);
    }
    
    // Benchmark
    let start = Instant::now();
    let mut high_confidence_count = 0;
    let mut total_confidence = 0.0;
    
    for line in &lines {
        let result = logoscope::smart_masking::smart_mask_line(line);
        if result.confidence > 0.8 {
            high_confidence_count += 1;
        }
        total_confidence += result.confidence;
    }
    
    let duration = start.elapsed();
    let lines_per_sec = lines.len() as f64 / duration.as_secs_f64();
    let avg_confidence = total_confidence / lines.len() as f64;
    
    println!("Results:");
    println!("  Total time: {:.3}s", duration.as_secs_f64());
    println!("  Lines per second: {:.0}", lines_per_sec);
    println!("  High confidence matches: {} / {} ({:.1}%)", 
             high_confidence_count, lines.len(), 
             high_confidence_count as f64 / lines.len() as f64 * 100.0);
    println!("  Average confidence: {:.3}", avg_confidence);
    
    Ok(())
}