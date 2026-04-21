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
    let db_sql = assets_root.join("db.sql");
    let templates_root = assets_root.join("templates");
    let migrations_root = assets_root.join("migrations");

    println!("cargo:rerun-if-changed={}", db_sql.display());
    println!("cargo:rerun-if-changed={}", templates_root.display());
    println!("cargo:rerun-if-changed={}", migrations_root.display());
    println!("cargo:rerun-if-changed={}", builtins_root.display());

    let template_assets = collect_assets(&templates_root, &templates_root);
    let migration_assets = collect_assets(&migrations_root, &migrations_root);
    let builtin_assets = collect_assets(&builtins_root, &builtins_root);

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    fs::write(
        out_dir.join("generated_assets.rs"),
        render_assets_module(
            &db_sql,
            &template_assets,
            &migration_assets,
            &builtin_assets,
        ),
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
    db_sql: &Path,
    template_assets: &[(String, String)],
    migration_assets: &[(String, String)],
    builtin_assets: &[(String, String)],
) -> String {
    let mut code = String::new();
    let db_sql = db_sql.to_string_lossy().replace('\\', "/");
    code.push_str(&format!(
        "pub static DB_SQL: &str = include_str!(r#\"{db_sql}\"#);\n\n"
    ));

    code.push_str("pub static TEMPLATE_ASSETS: &[(&str, &str)] = &[\n");
    for (relative, absolute) in template_assets {
        code.push_str(&format!(
            "    ({relative:?}, include_str!(r#\"{absolute}\"#)),\n"
        ));
    }
    code.push_str("];\n\n");

    code.push_str("pub static MIGRATION_ASSETS: &[(&str, &str)] = &[\n");
    for (relative, absolute) in migration_assets {
        code.push_str(&format!(
            "    ({relative:?}, include_str!(r#\"{absolute}\"#)),\n"
        ));
    }
    code.push_str("];\n");

    code.push_str("\npub static BUILTIN_ASSETS: &[(&str, &str)] = &[\n");
    for (relative, absolute) in builtin_assets {
        code.push_str(&format!(
            "    ({relative:?}, include_str!(r#\"{absolute}\"#)),\n"
        ));
    }
    code.push_str("];\n");
    code
}
