{
  description = "Atelier TUI — Terminal Development Environment for the T Language";

  inputs = {
    nixpkgs.url = "github:rstats-on-nix/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        atelier-pkg = pkgs.rustPlatform.buildRustPackage {
          pname = "atelier";
          version = "0.1.0";
          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = [ pkgs.makeWrapper ];
        };
      in
      {
        packages.default = atelier-pkg;
        packages.atelier = atelier-pkg;

        apps.default = {
          type = "app";
          program = "${atelier-pkg}/bin/atelier";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            gcc
            neovim
            nano
            atelier-pkg
          ];

          shellHook = ''
            echo "═══════════════════════════════════════════════"
            echo "Atelier TUI Development Environment (Rust)"
            echo "═══════════════════════════════════════════════"
            echo ""
            echo "Available commands:"
            echo "  atelier              - Launch the Atelier TUI"
            echo "  cargo build          - Build the Rust binary"
            echo "  cargo run            - Run during development"
            echo ""
          '';
        };
      }
    );
}
