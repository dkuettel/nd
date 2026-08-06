/// behaves like `nd --silent run --build-if-missing -- ...`
fn main() {
    let args: Vec<String> = std::env::args().collect();
    nd::set_volume(true, true);
    nd::run(&nd::Flake::Default, &args[1..], true, false, false);
}
