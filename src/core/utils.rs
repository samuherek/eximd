use std::path::Path;

// this is the list of all available and image extensions that are allowed to check
pub const IMGS: &'static [&'static str] = &[
    "cr3", "bmp", "cr2", "dng", "heic", "jpeg", "jpg", "nef", "png", "raf", "raw", "rw2", "svg",
    "tif", "tiff", "webp",
];

pub const VIDEOS: &'static [&'static str] = &["avi", "m4v", "mov", "mp4", "mpg"];

fn get_ext(path: &Path) -> String {
    let ext = path.extension().unwrap_or_default().to_ascii_lowercase();
    ext.to_str().unwrap_or_default().to_string()
}

pub fn is_video_ext(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let ext = get_ext(path);
    VIDEOS.contains(&ext.as_str())
}

pub fn is_img_ext(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let ext = get_ext(path);
    IMGS.contains(&ext.as_str())
}

pub fn is_video(ext: &str) -> bool {
    let ext = ext.to_lowercase();
    let ext = ext.as_str();
    VIDEOS.contains(&ext)
}

pub fn is_img(ext: &str) -> bool {
    let ext = ext.to_lowercase();
    let ext = ext.as_str();
    IMGS.contains(&ext)
}

pub fn is_primary_ext(ext: &str) -> bool {
    is_img(ext) || is_video(ext)
}


pub fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
