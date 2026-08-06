/// behaves like 'nd --silent run --maybe-env --build-if-missing -- $@`
/// and is a good candidate for running things in tmux mappings
/// > bind-key -T g g run-shell -b 'nd-tmux-run something --opt arg1 arg2'
fn main() {
    let args: Vec<String> = std::env::args().collect();
    nd::set_volume(true, true);
    nd::run(&nd::Flake::MaybeEnv, &args[1..], true, false, false);
}
