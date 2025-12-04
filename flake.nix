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
