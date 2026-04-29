pub fn trace(msg: &str) {
    ic0::debug_print(msg.as_bytes());
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
mod tests {}
