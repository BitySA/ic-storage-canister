pub fn trace(msg: &str) {
    ic0::debug_print(msg.as_bytes());
}

pub const MAX_FILE_PATH_LEN: usize = 512;

/// Validate a file_path before it becomes a storage key or part of a certified URL.
///
/// Why these rules:
/// - Length bound prevents a caller from inflating the HashMap key footprint.
/// - Control chars (\0 \r \n) and ?/# would split or escape URLs once the path
///   is interpolated into the response.
/// - `..` segments would let a caller alias paths after string normalization.
pub fn validate_file_path(p: &str) -> Result<(), &'static str> {
    if p.is_empty() {
        return Err("file_path is empty");
    }
    if p.len() > MAX_FILE_PATH_LEN {
        return Err("file_path too long");
    }
    for ch in p.chars() {
        match ch {
            '\0' | '\r' | '\n' | '?' | '#' => return Err("file_path contains forbidden character"),
            c if c.is_control() => return Err("file_path contains control character"),
            _ => {}
        }
    }
    // Strip a single leading '/' for segment validation; canonical storage form is no leading '/'.
    let stripped = p.strip_prefix('/').unwrap_or(p);
    for seg in stripped.split('/') {
        if seg == ".." {
            return Err("file_path contains '..' segment");
        }
    }
    Ok(())
}

pub fn get_content_type_for_path(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        // video
        "mp4" => "video/mp4",
        "m4v" => "video/x-m4v",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "mkv" => "video/x-matroska",
        "3gp" => "video/3gpp",
        "3g2" => "video/3gpp2",
        "ts" => "video/mp2t",

        // streaming
        // "m3u8" => "application/vnd.apple.mpegurl",

        // audio
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "m4a" => "audio/mp4",
        "ogg" => "audio/ogg",

        // images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",

        // documents
        "pdf" => "application/pdf",
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" => "text/plain",

        // fonts
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "eot" => "application/vnd.ms-fontobject",
        "otf" => "font/otf",

        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_file_path_accepts_simple_paths() {
        assert!(validate_file_path("/cat.png").is_ok());
        assert!(validate_file_path("cat.png").is_ok());
        assert!(validate_file_path("/dir/sub/file.mp4").is_ok());
        assert!(validate_file_path("a").is_ok());
    }

    #[test]
    fn validate_file_path_rejects_empty() {
        assert!(validate_file_path("").is_err());
    }

    #[test]
    fn validate_file_path_rejects_traversal() {
        assert!(validate_file_path("/../etc/passwd").is_err());
        assert!(validate_file_path("a/../b").is_err());
        assert!(validate_file_path("..").is_err());
    }

    #[test]
    fn validate_file_path_allows_dotfiles_and_dotdot_in_filename() {
        // Segment-level check, so ".." as a segment is rejected but a name
        // containing dots is fine.
        assert!(validate_file_path("/.hidden").is_ok());
        assert!(validate_file_path("/foo..bar").is_ok());
        assert!(validate_file_path("/foo.bar.baz").is_ok());
    }

    #[test]
    fn validate_file_path_rejects_control_chars() {
        assert!(validate_file_path("/foo\0bar").is_err());
        assert!(validate_file_path("/foo\rbar").is_err());
        assert!(validate_file_path("/foo\nbar").is_err());
        assert!(validate_file_path("/foo\u{0007}bar").is_err());
    }

    #[test]
    fn validate_file_path_rejects_url_metacharacters() {
        assert!(validate_file_path("/foo?bar").is_err());
        assert!(validate_file_path("/foo#bar").is_err());
    }

    #[test]
    fn validate_file_path_enforces_length_limit() {
        let ok_path = "/".to_string() + &"a".repeat(MAX_FILE_PATH_LEN - 1);
        assert!(validate_file_path(&ok_path).is_ok());

        let too_long = "/".to_string() + &"a".repeat(MAX_FILE_PATH_LEN);
        assert!(validate_file_path(&too_long).is_err());
    }
}
