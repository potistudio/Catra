fn main() {
    configure_windows_openssl_link();
    tauri_build::build();
}

fn configure_windows_openssl_link() {
    #[cfg(not(windows))]
    return;

    #[cfg(windows)]
    {
        use std::env;
        use std::path::PathBuf;

        let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
        let runtime = if profile.eq_ignore_ascii_case("release") {
            "MD"
        } else {
            "MDd"
        };

        let root = env::var("OPENSSL_DIR")
            .ok()
            .map(PathBuf::from)
            .filter(|path| path.is_dir())
            .or_else(|| {
                let path = PathBuf::from(r"C:\Program Files\OpenSSL-Win64");
                path.is_dir().then_some(path)
            });

        let Some(root) = root else {
            return;
        };

        let lib_dir = root.join("lib").join("VC").join("x64").join(runtime);
        if !lib_dir.join("libcrypto.lib").is_file() {
            println!(
                "cargo:warning=OpenSSL import lib not found at {}",
                lib_dir.display()
            );
            return;
        }

        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=dylib=libcrypto");
        println!("cargo:rerun-if-env-changed=OPENSSL_DIR");
        println!("cargo:rerun-if-env-changed=PROFILE");

        // Help rebuild sqlcipher if env was missing on a prior compile.
        env::set_var("OPENSSL_DIR", &root);
        env::set_var("OPENSSL_INCLUDE_DIR", root.join("include"));
        env::set_var("OPENSSL_LIB_DIR", &lib_dir);
    }
}
