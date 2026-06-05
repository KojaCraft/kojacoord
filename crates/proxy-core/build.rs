use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../dashboard-ui/src");
    println!("cargo:rerun-if-changed=../../dashboard-ui/package.json");

    let dashboard_dir = Path::new("../../dashboard-ui");
    if !dashboard_dir.exists() {
        println!("Dashboard UI directory not found, skipping build");
        return;
    }

    if env::var("KOJA_SKIP_DASHBOARD_BUILD").is_ok() {
        println!("Skipping dashboard build (KOJA_SKIP_DASHBOARD_BUILD set)");
        return;
    }

    println!("Building dashboard UI...");

    let npm_check = Command::new("npm").arg("--version").output();

    if npm_check.is_err() {
        println!("npm not found, skipping dashboard build");
        return;
    }

    if !dashboard_dir.join("node_modules").exists() {
        println!("Installing dashboard dependencies...");
        let install_status = Command::new("npm")
            .current_dir(dashboard_dir)
            .arg("install")
            .status();

        match install_status {
            Ok(status) if status.success() => {},
            Ok(_) => {
                println!("npm install failed, skipping dashboard build");
                return;
            },
            Err(e) => {
                println!("Failed to run npm install: {}, skipping dashboard build", e);
                return;
            },
        }
    }

    let build_status = Command::new("npm")
        .current_dir(dashboard_dir)
        .arg("run")
        .arg("build")
        .status();

    match build_status {
        Ok(status) if status.success() => {
            println!("Dashboard UI built successfully");
        },
        Ok(_) => {
            println!("Dashboard build failed, continuing without dashboard");
        },
        Err(e) => {
            println!(
                "Failed to build dashboard: {}, continuing without dashboard",
                e
            );
        },
    }
}
