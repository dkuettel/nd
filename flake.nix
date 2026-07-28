{
  description = "POC for an easy and fast `nix develop` workflow.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-26.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
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
        overlays = [
          (import rust-overlay)
          (self: super: {
            rustToolchain = pkgs.symlinkJoin {
              name = "rust-toolchain";
              paths = [
                (super.rust-bin.stable.latest.minimal.override {
                  extensions = [
                    "clippy"
                    "rust-analyzer"
                    "rust-docs"
                    "rust-src"
                  ];
                })
                (super.rust-bin.selectLatestNightlyWith (toolchain: toolchain.rustfmt))
              ];
            };
          })
        ];
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
        devShells.default = pkgs.mkShellNoCC {
          packages = with pkgs; [
            nil # nix language server
            nixfmt-rfc-style # nixpkgs-fmt is deprecated
            rustToolchain
            # TODO what are the next 4 for? check with yves
            pkg-config
            cargo-deny
            cargo-edit
            cargo-watch
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
