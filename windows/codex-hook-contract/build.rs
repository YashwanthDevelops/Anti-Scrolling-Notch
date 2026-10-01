use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const PINNED_CLI_VERSION: &str = "0.157.1";
const PINNED_SOURCE_TAG: &str = "rust-v0.157.1";
const PINNED_SOURCE_COMMIT: &str = "36650394c5b38c2990ccf2a3457165ca3e9d9726";

fn main() {
    let crate_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("crate directory"));
    let manifest_path = crate_dir.join("schema/release-0.157.1-manifest.json");
    println!("cargo:rerun-if-changed={}", manifest_path.display());

    let manifest: Value = serde_json::from_slice(
        &fs::read(&manifest_path).expect("read pinned Codex hook schema manifest"),
    )
    .expect("parse pinned Codex hook schema manifest");
    let cli_version = required_string(&manifest, "installed_cli_version");
    let source_commit = required_string(&manifest, "source_commit");
    assert_eq!(
        cli_version, PINNED_CLI_VERSION,
        "unexpected installed CLI version"
    );
    assert_eq!(
        required_string(&manifest, "source_tag"),
        PINNED_SOURCE_TAG,
        "unexpected Codex schema source tag"
    );
    assert_eq!(
        source_commit, PINNED_SOURCE_COMMIT,
        "unexpected Codex schema source commit"
    );
    let schemas = manifest["adapter_input_schemas"]
        .as_array()
        .expect("adapter_input_schemas must be an array");
    assert!(
        !schemas.is_empty(),
        "at least one adapter schema is required"
    );

    let schema_dir = crate_dir.join("schema/rust-v0.157.1");
    let mut events = Vec::with_capacity(schemas.len());
    for entry in schemas {
        let file = required_string(entry, "file");
        assert_eq!(
            Path::new(file).file_name().and_then(|name| name.to_str()),
            Some(file),
            "schema paths must be file names within the pinned schema directory"
        );
        let path = schema_dir.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = fs::read(&path).unwrap_or_else(|error| {
            panic!("read pinned hook input schema {}: {error}", path.display())
        });
        let expected_sha256 = required_string(entry, "sha256");
        let actual_sha256 = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(
            actual_sha256, expected_sha256,
            "pinned hook schema checksum mismatch for {file}"
        );

        let schema: Value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("parse pinned hook schema {file}: {error}"));
        let event = schema["properties"]["hook_event_name"]["const"]
            .as_str()
            .unwrap_or_else(|| panic!("{file} must pin hook_event_name.const"));
        assert!(
            !event.is_empty()
                && event
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
                && event
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_uppercase()),
            "hook event discriminator is not a supported Rust variant: {event}"
        );
        assert!(
            !events
                .iter()
                .any(|(known, _): &(String, String)| known == event),
            "duplicate hook event discriminator in pinned schemas: {event}"
        );
        events.push((event.to_owned(), file.to_owned()));
    }

    let mut generated = String::from(
        "#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]\n\
         pub enum CodexHookEvent {\n",
    );
    for (event, _) in &events {
        generated.push_str(&format!(
            "    #[serde(rename = \"{event}\")]\n    {event},\n"
        ));
    }
    generated.push_str("}\n\nimpl CodexHookEvent {\n");
    generated.push_str(&format!(
        "    pub const VERIFIED: [Self; {}] = [\n",
        events.len()
    ));
    for (event, _) in &events {
        generated.push_str(&format!("        Self::{event},\n"));
    }
    generated.push_str(
        "    ];\n\n    pub const fn as_str(self) -> &'static str {\n        match self {\n",
    );
    for (event, _) in &events {
        generated.push_str(&format!("            Self::{event} => \"{event}\",\n"));
    }
    generated.push_str(
        "        }\n    }\n\n    fn parse(value: &str) -> Option<Self> {\n        match value {\n",
    );
    for (event, _) in &events {
        generated.push_str(&format!(
            "            \"{event}\" => Some(Self::{event}),\n"
        ));
    }
    generated.push_str("            _ => None,\n        }\n    }\n}\n\n");
    generated.push_str(&format!(
        "pub const CODEX_HOOK_SCHEMA_CLI_VERSION: &str = \"{cli_version}\";\n"
    ));
    generated.push_str(&format!(
        "pub const CODEX_HOOK_SCHEMA_SOURCE_COMMIT: &str = \"{source_commit}\";\n"
    ));

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"))
        .join("codex_hook_event.rs");
    fs::write(output, generated).expect("write generated Codex hook event type");
}

fn required_string<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("pinned schema manifest field {key} must be a string"))
}
