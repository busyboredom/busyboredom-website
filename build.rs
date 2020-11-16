use std::process::Command;
use walkdir::WalkDir;
use std::{io};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

fn main() {
    // Compile the wasm.
    Command::new("wasm-pack")
        .args(&["build", "wasm/", "--release", "--target", "web"])
        .status()
        .unwrap();

    update_urls().expect("Error while updating URLs.");

    println!("cargo:rerun-if-changed=*");
}

// Update URLs when files change, to enable efficient caching. 
fn update_urls() -> Result<(), io::Error> {
    for entry in WalkDir::new("static")
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok()) {
        if entry.metadata().unwrap().is_file() {
            let f_name = entry.file_name().to_string_lossy();
            if !f_name.ends_with(".html") {
                if let Some(f_path) = entry.path().to_string_lossy().strip_prefix("static") {
                    println!("{}", f_path);
                    Command::new("ambs")
                        .args(&[f_path.to_owned() + "?ver="])
                        .status()
                        .unwrap();
                }
            }
        }
    }
    Ok(())
}

// Hashes a file given the file's path.
fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}