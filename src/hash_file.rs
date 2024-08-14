use sha2::Digest;
use sha2::Sha256;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;
use walkdir::WalkDir;

pub fn calc_hashing() -> Result<(), std::io::Error> {
    let path = "/Volumes/photo";
    let mut count = 0;
    let start = Instant::now();
    let mut hashes = vec![];

    for entry in WalkDir::new(path) {
        if count > 100 {
            break;
        }

        if count % 10 == 0 && count != 0 {
            println!("We are moving my friend {count}");
        }

        if let Ok(e) = entry {
            let p = e.path();
            println!("path: {:?}", p);
            if super::utils::is_video_ext(p) || super::utils::is_img_ext(p) {
                if let Ok(v) = hash_file(p) {
                    hashes.push(v);
                    count += 1;
                }
            }
        }
    }

    let duration = start.elapsed();
    println!("we got this many records: {count}");
    println!("time taken: {:?}", duration);

    let string_metadata_size = std::mem::size_of::<String>();
    let vector_metadata_size = std::mem::size_of::<Vec<String>>();
    let strings_heap_size: usize = hashes.iter().map(|s| s.capacity()).sum();
    let elements_size = hashes.len() * string_metadata_size + strings_heap_size;
    let total_size = elements_size + vector_metadata_size;
    println!("We have this much in memory: {total_size}");

    Ok(())
}

// Hash the file based on a path.
pub fn hash_file<P: AsRef<Path>>(path: P) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    let hash_result = hasher.finalize();
    Ok(format!("{:x}", hash_result))
}
