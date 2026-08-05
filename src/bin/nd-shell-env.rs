use std::env;

/// behaves like `nd --silent shell --env --build-if-missing -- $@`
fn main() {
    let args: Vec<String> = env::args().collect();
    nd::set_volume(true, true);
    nd::run_shell(&nd::Flake::Env, &args[1..], true, false, false);
}
