use std::path::Path;
use std::process::Command;

fn main() {
    if std::env::var("CARGO_FEATURE_DASHBOARD").is_err() {
        return;
    }

    println!("cargo:rerun-if-changed=dashboard/index.html");
    println!("cargo:rerun-if-changed=dashboard/src");
    println!("cargo:rerun-if-changed=dashboard/package.json");
    println!("cargo:rerun-if-changed=dashboard/vite.config.ts");
    println!("cargo:rerun-if-changed=dashboard/tsconfig.json");
    println!("cargo:rerun-if-changed=dashboard/tsconfig.app.json");
    println!("cargo:rerun-if-changed=dashboard/dist");

    let dashboard_dir = Path::new("dashboard");

    let bun = if cfg!(target_os = "windows") {
        "bun.exe"
    } else {
        "bun"
    };

    let status = Command::new(bun)
        .args(["install"])
        .current_dir(dashboard_dir)
        .status()
        .unwrap_or_else(|e| panic!("Failed to run `bun install`: {e}"));

    assert!(
        status.success(),
        "`bun install` exited with status {status}"
    );

    let status = Command::new(bun)
        .args(["run", "build"])
        .current_dir(dashboard_dir)
        .status()
        .unwrap_or_else(|e| panic!("Failed to run `bun run build`: {e}"));

    assert!(
        status.success(),
        "`bun run build` exited with status {status}"
    );
}
