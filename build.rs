use blake3;
use regex::Regex;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

const HASH_BUFFER_SIZE: usize = 16384;

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
        // If the entry is a file,
        if entry.metadata().unwrap().is_file() {
            // Strip the "static" part of its path.
            if let Some(f_path) = entry.path().to_string_lossy().strip_prefix("static") {
                // Open it.
                if let Ok(mut file) = File::open(entry.path()) {
                    // Hash it.
                    let hash = generate_hash::<_>(&mut file);
                    println!(
                        "Setting hash to \"{}\" for all instances of \"{}\".",
                        hash, f_path,
                    );
                    // Update its hash in all URLs pointing to it.
                    for dir in ["src/", "static/", "wasm/src/"].iter() {
                        set_url_hash(f_path, &hash, dir).expect("Unable to set URL hash");
                    }
                }
            }
        }
    }
    Ok(())
}

fn set_url_hash(resource: &str, hash: &str, directory: &str) -> Result<(), io::Error> {
    // Define pattern to search for.
    let from_regex = Regex::new(&(resource.replace(".", "\\.") + r"\?ver=[A-Za-z0-9_-]+"))
        .expect("Invalid Regex in URL generation.");

    for entry in WalkDir::new(directory)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry
            .metadata()
            .expect("Unable to read metadata during URL setting/generation")
            .is_file()
        {
            if let Some(path_str) = entry.path().to_str() {
                let extension = entry.path().extension().unwrap();
                if extension == "html" || extension == "rs" {
                    let file_path = Path::new(path_str);
                    //let file_path = Path::new("static/resume.html");
                    // Open and read the file entirely
                    let mut src = File::open(&file_path)?;
                    let mut data = String::new();
                    src.read_to_string(&mut data)?;
                    drop(src); // Close the file early

                    let new_data =
                        from_regex.replace_all(&data, &*(resource.to_owned() + "?ver=" + hash));

                    // Recreate the file and dump the processed contents to it
                    let mut dst = File::create(&file_path)?;
                    dst.write_all(new_data.as_bytes())?;
                }
            }
        }
    }

    Ok(())
}

fn generate_hash<R: Read>(reader: &mut R) -> String {
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; HASH_BUFFER_SIZE];
    loop {
        let n = match reader.read(&mut buffer) {
            Ok(n) => n,
            Err(_) => panic!("Unable to read file while attempting to generate hash"),
        };
        hasher.update(&buffer[..n]);
        if n == 0 || n < HASH_BUFFER_SIZE {
            break;
        }
    }
    let mut output = [0; 8];
    let mut output_reader = hasher.finalize_xof();
    output_reader.fill(&mut output);

    // Convert to unpadded base64 and return.
    base64::encode_config(output, base64::URL_SAFE.pad(false))
}
