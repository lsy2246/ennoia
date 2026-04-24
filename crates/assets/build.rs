use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("repo root");
    let assets_root = repo_root.join("assets");
    let builtins_root = repo_root.join("builtins");
    let templates_root = assets_root.join("templates");

    println!("cargo:rerun-if-changed={}", templates_root.display());
    println!("cargo:rerun-if-changed={}", builtins_root.display());

    let template_assets = collect_assets(&templates_root, &templates_root);
    let builtin_assets = collect_assets(&builtins_root, &builtins_root);

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    fs::write(
        out_dir.join("generated_assets.rs"),
        render_assets_module(&template_assets, &builtin_assets),
    )
    .expect("write generated assets");
}

fn collect_assets(root: &Path, current: &Path) -> Vec<(String, String)> {
    let mut assets = Vec::new();
    if !current.exists() {
        return assets;
    }

    let mut entries = fs::read_dir(current)
        .expect("read asset dir")
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            assets.extend(collect_assets(root, &path));
            continue;
        }

        println!("cargo:rerun-if-changed={}", path.display());

        let relative = path
            .strip_prefix(root)
            .expect("asset under root")
            .to_string_lossy()
            .replace('\\', "/");
        let absolute = path.to_string_lossy().replace('\\', "/");
        assets.push((relative, absolute));
    }

    assets
}

fn render_assets_module(
    template_assets: &[(String, String)],
    builtin_assets: &[(String, String)],
) -> String {
    let (builtin_text_assets, builtin_binary_assets): (Vec<_>, Vec<_>) = builtin_assets
        .iter()
        .cloned()
        .partition(|(relative, _)| !is_binary_asset(relative));
    let mut code = String::new();
    code.push_str("pub static TEMPLATE_ASSETS: &[(&str, &str)] = &[\n");
    for (relative, absolute) in template_assets {
        code.push_str(&format!(
            "    ({relative:?}, include_str!(r#\"{absolute}\"#)),\n"
        ));
    }
    code.push_str("];\n");

    code.push_str("\npub static BUILTIN_ASSETS: &[(&str, &str)] = &[\n");
    for (relative, absolute) in &builtin_text_assets {
        code.push_str(&format!(
            "    ({relative:?}, include_str!(r#\"{absolute}\"#)),\n"
        ));
    }
    code.push_str("];\n");

    code.push_str("\npub static BUILTIN_BINARY_ASSETS: &[(&str, &[u8])] = &[\n");
    for (relative, absolute) in &builtin_binary_assets {
        code.push_str(&format!(
            "    ({relative:?}, include_bytes!(r#\"{absolute}\"#)),\n"
        ));
    }
    code.push_str("];\n");
    code
}

fn is_binary_asset(path: &str) -> bool {
    path.ends_with(".wasm")
        || path.ends_with(".exe")
        || matches!(
            path.split('/').collect::<Vec<_>>().as_slice(),
            ["extensions", _, "bin", ..]
        )
}
