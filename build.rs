fn main() {
    // Linux: fltk-sys 的 bundled 构建在某些环境下会编进 Cairo 相关代码，但链接阶段未自动补齐 cairo/gobject 库。
    // 这里通过 pkg-config 显式补齐所需的 -L/-l，避免 CI 上出现 undefined symbol: cairo_* / g_object_unref。
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR");

        if let Ok(output) = std::process::Command::new("pkg-config")
            .args(["--libs", "cairo", "gobject-2.0", "pangocairo"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for token in stdout.split_whitespace() {
                    if let Some(path) = token.strip_prefix("-L") {
                        println!("cargo:rustc-link-search=native={}", path);
                    } else if let Some(lib) = token.strip_prefix("-l") {
                        println!("cargo:rustc-link-lib={}", lib);
                    }
                }
            }
        }
    }

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