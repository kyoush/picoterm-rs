use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Determine which memory layout to use based on features
    let memory_file = if env::var("CARGO_FEATURE_RP2350").is_ok() {
        "memory_rp2350.x"
    } else {
        // Default to RP2040
        "memory_rp2040.x"
    };

    // Copy the appropriate memory file to memory.x in the project root
    let src = Path::new(memory_file);
    let dst = Path::new("memory.x");

    fs::copy(&src, &dst).expect("Failed to copy memory file");

    // Tell cargo to rerun if the memory files change
    println!("cargo:rerun-if-changed={}", memory_file);
    println!("cargo:rerun-if-changed=memory_rp2040.x");
    println!("cargo:rerun-if-changed=memory_rp2350.x");
}
