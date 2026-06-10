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

        atelier-pkg = pkgs.stdenv.mkDerivation {
          pname = "atelier";
          version = "0.1.0";
          src = ./.;

          nativeBuildInputs = [ pkgs.makeWrapper ];

          buildInputs = [
            pkgs.python3
            pkgs.tmux
            pkgs.neovim
          ];

          installPhase = ''
            mkdir -p $out/bin $out/lib/atelier
            cp atelier-vars.py $out/lib/atelier/
            cp atelier-watcher.sh $out/lib/atelier/
            cp atelier-init.lua $out/lib/atelier/
            chmod +x $out/lib/atelier/atelier-vars.py
            chmod +x $out/lib/atelier/atelier-watcher.sh

            cp atelier $out/bin/atelier
            chmod +x $out/bin/atelier

            wrapProgram $out/bin/atelier \
              --prefix PATH : "${pkgs.lib.makeBinPath [ pkgs.tmux pkgs.neovim pkgs.python3 ]}" \
              --set ATELIER_DIR "$out/lib/atelier"
          '';
        };
      in
      {
        packages.default = atelier-pkg;
        packages.atelier = atelier-pkg;

        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.tmux
            pkgs.neovim
            pkgs.python3
            atelier-pkg
          ];

          shellHook = ''
            echo "═══════════════════════════════════════════════"
            echo "Atelier TUI Development Environment"
            echo "═══════════════════════════════════════════════"
            echo ""
            echo "Available commands:"
            echo "  atelier              - Launch the Atelier TUI"
            echo ""
          '';
        };
      }
    );
}
