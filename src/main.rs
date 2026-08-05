use clap::Parser;
use nd::{
    build, cli_run, is_volume_included, maybe_warn, resolve_flake, set_volume, uprintln, Cli,
    Commands, Volume,
};

fn main() {
    let args = Cli::parse();

    set_volume(args.quiet, args.silent);

    match args.command {
        Commands::Build { spec, if_missing } => {
            let flake = spec.as_flake();
            if let Some(at) = resolve_flake(&flake) {
                if if_missing && at.join(".nd/run").is_file() {
                    return;
                }
                build(&at);
            }
        }
        Commands::Run {
            spec,
            command,
            opts,
        } => {
            cli_run(&spec, &command, &opts);
        }
        Commands::Shell { spec, opts, args } => {
            let shell = std::env::var("SHELL").unwrap_or(String::from("sh"));
            let mut command = vec![shell];
            command.extend_from_slice(&args);
            cli_run(&spec, &command, &opts);
        }
        Commands::Info { spec } => {
            uprintln!(
                Volume::Normal,
                "$nd_env: {}",
                std::env::var("nd_env").unwrap_or("<not set>".into())
            );
            uprintln!(
                Volume::Normal,
                "$nd: {}",
                std::env::var("nd").unwrap_or("<not set>".into())
            );
            uprintln!(
                Volume::Normal,
                "$nd_nix: {}",
                std::env::var("nd_nix").unwrap_or("<not set>".into())
            );
            let flake = spec.as_flake();
            if let Some(at) = resolve_flake(&flake) {
                uprintln!(Volume::Normal, "Resolving to flake at: {}", at.display());
                maybe_warn(&at);
            } else {
                uprintln!(
                    Volume::Normal,
                    "Resolving to no flake as requested: Pass through mode."
                );
            }
        }
    };
}
