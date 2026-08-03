{
  description = "POC for an easy and fast `nix develop` workflow.";

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
      in
      {
        packages.default = pkgs.stdenv.mkDerivation {
          name = "nd";
          src = ./pkg;
          installPhase = ''
            cp -r $src $out
          '';
        };
        packages.shell = shell;
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            nil # nix language server
            nixfmt # nix formatter
            (rust-bin.stable.latest.default.override {
              # see https://rust-lang.github.io/rustup/concepts/components.html
              # and https://rust-lang.github.io/rustup/concepts/profiles.html
              extensions = [
                "rust-src"
                "rust-analyzer"
              ];
            })
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
  # TODO lets see if we can offer more useful integration
  # // {
  #   nixosModules.default =
  #     { config, ... }:
  #     {
  #       options = { };
  #       config = { };
  #     };
  #
  # };
}
