use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../dashboard-ui/src");
    println!("cargo:rerun-if-changed=../../dashboard-ui/package.json");
    println!("cargo:rerun-if-changed=proto/control_plane.proto");

    // Build dashboard UI
    let dashboard_dir = Path::new("../../dashboard-ui");
    if !dashboard_dir.exists() {
        println!("Dashboard UI directory not found, skipping build");
    } else {
        if env::var("KOJA_SKIP_DASHBOARD_BUILD").is_ok() {
            println!("Skipping dashboard build (KOJA_SKIP_DASHBOARD_BUILD set)");
        } else {
            println!("Building dashboard UI...");

            let npm_check = Command::new("npm").arg("--version").output();

            if npm_check.is_ok() {
                if !dashboard_dir.join("node_modules").exists() {
                    println!("Installing dashboard dependencies...");
                    let install_status = Command::new("npm")
                        .current_dir(dashboard_dir)
                        .arg("install")
                        .status();

                    if install_status.is_err() || !install_status.unwrap().success() {
                        println!("npm install failed, skipping dashboard build");
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
            } else {
                println!("npm not found, skipping dashboard build");
            }
        }
    }

    // Compile protobuf files
    tonic_build::configure()
        .build_server(true)
        .compile(&["proto/control_plane.proto"], &["proto/"])
        .expect("Failed to compile protobuf files");
}
