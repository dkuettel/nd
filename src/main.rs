use std::{
    fs::{self, create_dir_all},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
#[command(infer_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// build and cache current flake for later use
    #[command(visible_alias = "b")]
    Build {
        /// force use of flake from env var `nd_env`, and fail otherwise
        #[arg(short, long)]
        env: bool,
        /// use provided flake location, and fail otherwise
        #[arg(short, long)]
        flake: Option<PathBuf>,
    },

    /// run a command in the latest dev shell
    #[command(visible_alias = "r")]
    Run {},

    /// run an interactive dev shell
    #[command(visible_alias = "s")]
    Shell {},

    /// show general info
    #[command(visible_alias = "i")]
    Info {},
}

// TODO should I use Path when not building?
fn resolve_flake(env: bool, flake: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // TODO can clap do that for us?
    assert!(
        flake.is_none() || !env,
        "Cannot not use --env and --flake at the same time."
    );

    if let Ok(at) = std::env::var("nd_env") {
        return Ok(at.into());
    }

    if env {
        return Err("The env var `nd_env` should be set.".into());
    }

    if let Some(at) = flake {
        return Ok(at);
    }

    for dir in std::env::current_dir()?.ancestors() {
        let at = dir.join("flake.nix");
        if at.is_file() {
            return Ok(dir.into());
        }
    }

    Err("cannot find any flake".into())
}

fn build(folder: &Path) {
    let nd = folder.join(".nd");
    let profile = folder.join(".nd/dev");
    let run = folder.join(".nd/run");

    create_dir_all(nd).expect("Should be able to create the `.nd` folder.");

    let output = Command::new("nix")
        .arg("print-dev-env")
        .arg("--quiet")
        .arg("--quiet")
        .arg("--profile")
        .arg(&profile)
        .arg(folder)
        .output()
        .expect("Should run `nix print-dev-env` successfully.");

    let profile = profile
        .canonicalize()
        .expect("Should be able to follow `.nd/dev` symlinks to the nix store.");
    let profile_str = profile.to_str().unwrap();

    let mut script = String::new();
    script.push_str("#!/usr/bin/env bash\n");
    script.push('\n');
    script.push_str(
        str::from_utf8(&output.stdout).expect("`nix print-dev-env` should produce utf8 output."),
    );
    script.push_str("\n\n");
    script.push_str(format!("export nd_nix={}\n", profile_str).as_str());
    script.push_str("exec \"$@\"\n");

    fs::write(&run, script.as_str())
        .expect("Should have write access for the `.nd/run` script file.");

    fs::set_permissions(&run, fs::Permissions::from_mode(0o755))
        .expect("Should have write access for the `.nd/run` script file.");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    println!("{:?}", args);

    match args.command {
        Commands::Build { env, flake } => {
            let at = resolve_flake(env, flake)?;
            assert!(at.is_dir());
            println!("{:?}", at);
            build(&at);
        }
        Commands::Run {} => todo!(),
        Commands::Shell {} => todo!(),
        Commands::Info {} => todo!(),
    };

    Ok(())
}
