use regex::Regex;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

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
        .filter_map(|e| e.ok())
    {
        if entry.metadata().unwrap().is_file() {
            let f_name = entry.file_name().to_string_lossy();
            if !f_name.ends_with(".html") {
                if let Some(f_path) = entry.path().to_string_lossy().strip_prefix("static") {
                    println!("{}", f_path);
                    set_url_hash("/resume.pdf", "1").unwrap();
                }
            }
        }
    }
    Ok(())
}

fn set_url_hash(resource: &str, hash: &str) -> Result<(), io::Error> {
    let file_path = Path::new("static/resume.html");

    // Open and read the file entirely
    let mut src = File::open(&file_path)?;
    let mut data = String::new();
    src.read_to_string(&mut data)?;
    drop(src); // Close the file early

    let from_regex =
        Regex::new(r"/resume\.pdf\?ver=[A-Za-z0-9_-]+").expect("Invalid Regex in URL generation.");

    let new_data =
        from_regex.replace_all(&data, &*("/resume.pdf?ver=".to_owned() + hash));

    // Recreate the file and dump the processed contents to it
    let mut dst = File::create(&file_path)?;
    dst.write(new_data.as_bytes())?;

    println!(
        "Set hash for \"{}\" to \"{}\" in \"{}\".",
        resource,
        hash,
        file_path.display()
    );

    Ok(())
}
// grep -rl '\/resume\.pdf?ver=[A-Za-z0-9_-]\+' static/ src/ wasm/src/
// | xargs sed -i 's/\/resume\.pdf?ver=[A-Za-z0-9_-]\+/\/resume\.pdf?ver=1/g'
