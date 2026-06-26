fn main() {
    #[cfg(feature = "dashboard")]
    build_dashboard();
    #[cfg(feature = "wca-scrambles")]
    build_scrambles();
}

#[cfg(feature = "wca-scrambles")]
fn build_scrambles() {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    println!("cargo:rerun-if-changed=scrambles/lib/src/main/java/org/example/Library.java");
    println!("cargo:rerun-if-changed=scrambles/lib/src/test/java/org/example/LibraryTest.java");
    println!("cargo:rerun-if-changed=scrambles/lib/build.gradle.kts");

    let scrambles_dir = Path::new("scrambles");

    let mut command = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "gradlew.bat", "shadowJar", "--no-daemon"]);
        cmd
    } else {
        let mut cmd = Command::new("./gradlew");
        cmd.args(["shadowJar", "--no-daemon"]);
        cmd
    };

    let status = command
        .current_dir(scrambles_dir)
        .status()
        .unwrap_or_else(|e| panic!("Failed to run `gradle shadowJar`: {e}"));

    assert!(
        status.success(),
        "`gradle shadowJar` exited with status {status}"
    );

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let src_jar = scrambles_dir.join("lib/build/libs/lib-all.jar");
    let dst_jar = out_dir.join("lib-all.jar");
    fs::copy(&src_jar, &dst_jar).unwrap_or_else(|e| {
        panic!(
            "Failed to copy {} to {}: {e}",
            src_jar.display(),
            dst_jar.display()
        )
    });
}

#[cfg(feature = "dashboard")]
fn build_dashboard() {
    use std::path::Path;
    use std::process::Command;

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
