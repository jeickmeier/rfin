//! Namespace parity between finstack-wasm/index.js and the parity contract.
//!
//! This test reads `finstack-wasm/index.js` and asserts that the set of
//! top-level namespaces re-exported from `./exports/<name>.js` matches the
//! `[wasm_top_level].namespaces` list in `finstack-py/parity_contract.toml`.
//!
//! The companion `[pyi_top_level]` check in finstack-py covers the Python
//! root package. Together they form the cross-binding tripwire.

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("finstack-wasm has a parent (workspace root)")
        .to_path_buf()
}

fn read_index_js() -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(manifest_dir.join("index.js")).expect("read finstack-wasm/index.js")
}

fn read_parity_contract() -> String {
    let path = workspace_root().join("finstack-py").join("parity_contract.toml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e))
}

/// Extract `<name>` from lines like `export { <name> } from './exports/<file>';`.
fn parse_index_js_namespaces(source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("export { ") else { continue };
        let Some(name_end) = rest.find(" }") else { continue };
        let name = &rest[..name_end];
        if rest[name_end..].contains("from './exports/") {
            out.push(name.to_string());
        }
    }
    out.sort();
    out
}

/// Extract the `namespaces = [...]` list under `[wasm_top_level]` in the parity TOML.
fn parse_contract_namespaces(toml_text: &str) -> Vec<String> {
    let table: toml::Value = toml::from_str(toml_text).expect("parity_contract.toml is valid TOML");
    let block = table
        .get("wasm_top_level")
        .expect("[wasm_top_level] section present");
    let names = block
        .get("namespaces")
        .expect("`namespaces` key present")
        .as_array()
        .expect("`namespaces` is an array");
    let mut out: Vec<String> = names
        .iter()
        .map(|v| v.as_str().expect("each namespace is a string").to_string())
        .collect();
    out.sort();
    out
}

#[test]
fn index_js_top_level_matches_parity_contract() {
    let index = read_index_js();
    let contract = read_parity_contract();
    let actual = parse_index_js_namespaces(&index);
    let expected = parse_contract_namespaces(&contract);
    assert_eq!(
        actual, expected,
        "finstack-wasm/index.js top-level namespaces diverged from parity contract.\n\
         actual:   {actual:?}\n\
         expected: {expected:?}"
    );
}

#[test]
fn each_named_namespace_has_an_exports_file() {
    let contract = read_parity_contract();
    let namespaces = parse_contract_namespaces(&contract);
    let exports_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("exports");
    for ns in namespaces {
        let path = exports_dir.join(format!("{ns}.js"));
        assert!(
            path.exists(),
            "contract lists `{ns}` but {} does not exist",
            path.display()
        );
    }
}
