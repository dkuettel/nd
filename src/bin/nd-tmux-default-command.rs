/// behaves like `nd --silent shell --maybe-env --build-if-missing --warn -- $@`
/// and should be a good candidate for tmux:
/// > set-option -g default-command nd-tmux-default-command
fn main() {
    nd::set_volume(true, true);
    nd::run_shell(&nd::Flake::MaybeEnv, &[], true, true, false);
}
