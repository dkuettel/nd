use std::env;

/// behaves like `nd --quiet shell --env --build-if-missing -- $@`
fn main() {
    let args: Vec<String> = env::args().collect();
    nd::set_volume(true, true);
    // TODO we dont want to work on this level
    let spec = nd::FlakeSpec {
        any: false,
        here: false,
        env: true,
        at: None,
    };
    let shell = std::env::var("SHELL").unwrap_or(String::from("sh"));
    let mut command = vec![shell];
    command.extend_from_slice(&args[1..]);
    let opts = nd::RunOpts {
        build_if_missing: true,
        warn: false,
        build: false,
    };
    nd::cli_run(&spec, &command, &opts);
}
