{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

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
        hyprlauncher = pkgs.callPackage ./default.nix { src = self; };
      in
      {
        packages = {
          inherit hyprlauncher;
          default = hyprlauncher;
        };
        devShells.default = pkgs.mkShell {
          name = "hyprlauncher";
          inputsFrom = [
            hyprlauncher
          ];
          packages = with pkgs; [
            clippy
            rustfmt
            rust-analyzer
          ];
        };
      }
    );
}
