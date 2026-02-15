use std::{env, path::PathBuf, process::Command};

pub fn main() {
    println!("cargo:rerun-if-env-changed=SKIP_FRONTEND_BUILD");

    let skip = env::var_os("CARGO_FEATURE_EMBED_FRONTEND").is_none();

    if skip {
        return;
    }

    println!("cargo:rerun-if-changed=web/pnpm-lock.yaml");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/src");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let frontend_dir = manifest_dir.join("web");

    // pnpm install
    run(&frontend_dir, "pnpm", &["install"]);

    // pnpm build
    run(&frontend_dir, "pnpm", &["build"]);
}

fn run(cwd: &PathBuf, program: &str, args: &[&str]) {
    let status = Command::new(program)
        .current_dir(cwd)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("failed to execute {program}: {e}"));
    if !status.success() {
        panic!("{program} {:?} failed with status {status}", args);
    }
}
