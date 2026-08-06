{
  description = "nd - a fast nix develop wrapper";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-26.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      # see https://github.com/oxalica/rust-overlay
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        shell = pkgs.runCommandLocal "shell" { } ''
          mkdir -p $out
          ln -sfT ${./share} $out/share
        '';
        rustToolchain = (
          pkgs.rust-bin.stable.latest.default.override {
            # see https://rust-lang.github.io/rustup/concepts/components.html
            # and https://rust-lang.github.io/rustup/concepts/profiles.html
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
          }
        );
        pkg = pkgs.rustPlatform.buildRustPackage {
          pname = "nd";
          version = "1.0.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = [
            rustToolchain
          ];
          meta = {
            description = "nd - a fast nix develop wrapper";
            homepage = "https://github.com/dkuettel/nd";
            license = pkgs.lib.licenses.mit;
            mainProgram = "nd";
          };
        };
      in
      {
        packages.default = pkg;
        # TODO this is non-standard, would be cool to setup some `nixosModules.default = ...` to have it easy for managed zsh?
        packages.shell = shell;
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            nil # nix language server
            nixfmt # nix formatter
            rustToolchain
          ];
          shellHook = ''
            if [[ -v h ]]; then
              export PATH=$h/bin:$PATH;
            else
              echo 'Project root env var h is not set.' >&2
            fi
          '';
        };
      }
    );
}
