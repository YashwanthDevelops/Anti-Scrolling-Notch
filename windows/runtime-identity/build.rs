use std::{env, fs, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"))
        .parent()
        .expect("runtime-identity package is under windows")
        .join("runtime-identity.json");
    println!("cargo:rerun-if-changed={}", manifest.display());

    let values: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).expect("read runtime identity manifest"))
            .expect("parse runtime identity manifest");
    let mappings = [
        ("PRODUCT_NAME", "productName"),
        ("APP_IDENTIFIER", "appIdentifier"),
        ("APP_CARGO_PACKAGE", "appCargoPackage"),
        ("APP_LIBRARY", "appLibrary"),
        ("FRONTEND_PACKAGE", "frontendPackage"),
        ("RELAY_PACKAGE", "relayPackage"),
        ("RELAY_EXECUTABLE", "relayExecutable"),
        ("STORAGE_DIRECTORY", "storageDirectory"),
        ("CREDENTIAL_NAMESPACE", "credentialNamespace"),
        ("AUTOSTART_VALUE_NAME", "autostartValueName"),
        ("APP_PIPE_PREFIX", "appPipePrefix"),
        ("CODEX_PIPE_PREFIX", "codexPipePrefix"),
        ("LOG_FILE_NAME", "logFileName"),
    ];

    let mut generated = String::from("// Generated from windows/runtime-identity.json.\n");
    for (constant, key) in mappings {
        let value = values[key]
            .as_str()
            .unwrap_or_else(|| panic!("runtime identity field {key} must be a string"));
        generated.push_str(&format!("pub const {constant}: &str = {value:?};\n"));
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("identity.rs");
    fs::write(out, generated).expect("write generated runtime identity constants");
}
