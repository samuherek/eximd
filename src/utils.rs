use std::path::Path;

// This is the list of all the found extensions.
const EXTS: &'static [&'static str] = &[
    "3gp",
    "aae",
    "ai",
    "avi",
    "bmp",
    "bup",
    "caf",
    "cr2",
    "cr3",
    "csv",
    "db",
    "dng",
    "fcpevent",
    "flexolibrary",
    "gif",
    "heic",
    "ifo",
    "ini",
    "jpeg",
    "jpg",
    "js",
    "json",
    "lua",
    "m4v",
    "map",
    "mov",
    "mp3",
    "mp4",
    "mpg",
    "nef",
    "pages",
    "pdf",
    "plist",
    "png",
    "psd",
    "raf",
    "raw",
    "rtf",
    "rw2",
    "svg",
    "thm",
    "tif",
    "tiff",
    "txt",
    "url",
    "vob",
    "wav",
    "webp",
    "xmp",
    "zip",
];

// this is the list of all available and image extensions that are allowed to check
pub const IMGS: &'static [&'static str] = &[
    "cr3", "bmp", "cr2", "dng", "heic", "jpeg", "jpg", "nef", "png", "raf", "raw", "rw2", "svg",
    "tif", "tiff", "webp",
];

pub const VIDEOS: &'static [&'static str] = &["avi", "m4v", "mov", "mp4", "mpg"];

fn get_ext(path: &Path) -> String {
    let ext = path
        .extension()
        .unwrap_or(std::ffi::OsStr::new("xxx"))
        .to_ascii_lowercase();
    ext.to_str().unwrap_or("xxx").to_string()
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
