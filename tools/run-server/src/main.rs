use std::env;
use std::fs;
use std::path::Path;
use std::process::{self, Command, ExitStatus};

fn main() {
    match run() {
        Ok(status) => process::exit(status.code().unwrap_or(1)),
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

fn run() -> Result<ExitStatus, String> {
    let project_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let server = env::var_os("PUMPKIN_EXE").map_or_else(
        || {
            project_dir
                .join("server")
                .join(format!("pumpkin{}", env::consts::EXE_SUFFIX))
        },
        std::path::PathBuf::from,
    );
    let server = server.canonicalize().map_err(|error| {
        format!(
            "Cannot find Pumpkin at {} ({error}). Set PUMPKIN_EXE to its executable path.",
            server.display()
        )
    })?;
    if !server.is_file() {
        return Err(format!(
            "Pumpkin executable is not a file: {}",
            server.display()
        ));
    }
    let server_dir = server.parent().expect("a canonical file path has a parent");

    let target_dir = project_dir.join("target");
    let mut build = Command::new(env!("CARGO"));
    build.current_dir(&project_dir).arg("build-plugin");
    build.arg("--target-dir").arg(&target_dir);
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        build.arg("--release");
        "release"
    };
    let status = build
        .status()
        .map_err(|error| format!("Cannot run Cargo: {error}"))?;
    if !status.success() {
        return Ok(status);
    }

    let plugin = target_dir
        .join("wasm32-wasip2")
        .join(profile)
        .join("rummo.wasm");
    let plugins_dir = server_dir.join("plugins");
    fs::create_dir_all(&plugins_dir)
        .map_err(|error| format!("Cannot create {}: {error}", plugins_dir.display()))?;
    let destination = plugins_dir.join("rummo.wasm");
    fs::copy(&plugin, &destination).map_err(|error| {
        format!(
            "Cannot copy {} to {}: {error}",
            plugin.display(),
            destination.display()
        )
    })?;

    Command::new(&server)
        .current_dir(server_dir)
        .args(env::args_os().skip(1))
        .status()
        .map_err(|error| format!("Cannot start {}: {error}", server.display()))
}
