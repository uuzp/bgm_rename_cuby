fn main() {
    if std::env::var_os("CARGO_FEATURE_ICON").is_some() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let p1 = std::path::Path::new(&manifest_dir).join("images").join("ico.rc");
        let p2 = std::path::Path::new(&manifest_dir)
            .join("..")
            .join("images")
            .join("ico.rc");

        let path = if p1.exists() { p1 } else { p2 };
        let _ = embed_resource::compile(path.to_string_lossy().as_ref(), embed_resource::NONE);
    }
}