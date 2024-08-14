use std::path::Path;
use walkdir::WalkDir;

// Collect the list of all the extensions that we have within a directory recursivelly.
pub fn get_list_of_extensions(path: impl AsRef<Path>) -> Result<Vec<String>, std::io::Error> {
    let mut count = 0;
    let mut ext_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in WalkDir::new(path) {
        if count % 1000 == 0 {
            println!("Total in batch {count}");
        }
        // if count > 10000 {
        //     break;
        // }
        if let Ok(e) = entry {
            let p = e.path();
            if p.is_file() {
                if let Some(ext) = p.extension() {
                    ext_set.insert(ext.to_string_lossy().to_string());
                }
            }
        }
        count += 1;
    }

    Ok(ext_set.into_iter().collect())
}
