{
  description = "POC for an easy and fast `nix develop` workflow.";

  inputs = {
    nixpkgs.url = "github:dkuettel/nixpkgs/stable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.stdenv.mkDerivation {
          name = "nd";
          src = ./pkg;
          installPhase = ''
            cp -r $src $out
          '';
        };
        devShells.default = pkgs.mkShellNoCC {
          packages = with pkgs; [
            nil # nix language server
            nixfmt-rfc-style # nixpkgs-fmt is deprecated
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
