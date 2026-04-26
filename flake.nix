{
  description = "Bouldy: Rust runtime modding MVP for Unreal Engine games";

  inputs = {
    rs-harbor.url = "git+ssh://git@codeberg.org/caniko/rs-harbor.git";

    nixpkgs.follows = "rs-harbor/nixpkgs";
    rust-overlay.follows = "rs-harbor/rust-overlay";
    crane.follows = "rs-harbor/crane";
    flake-utils.follows = "rs-harbor/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rs-harbor,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };
      lib = pkgs.lib;

      toolchain = rs-harbor.lib.mkToolchain {
        inherit pkgs;
        channel = "stable";
        extensions = ["rust-src" "rustfmt" "clippy"];
        crossTargets = [
          "x86_64-unknown-linux-gnu"
          "x86_64-pc-windows-gnu"
        ];
      };

      cross = rs-harbor.lib.mkCross {
        inherit pkgs system;
      };

      cargoConfig = rs-harbor.lib.mkCargoConfig {
        inherit pkgs;
        channel = "stable";
        crossTargets = toolchain.crossTargets;
      };

      website = pkgs.stdenv.mkDerivation {
        pname = "bouldy-website";
        version = "0.1.0";
        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.maybeMissing ./website;
        };
        nativeBuildInputs = [pkgs.zola];
        phases = ["buildPhase" "installPhase"];
        buildPhase = ''
          cp -r --no-preserve=mode $src/website site
          cd site
          zola build
        '';
        installPhase = ''
          cp -r public $out
        '';
      };

      docs = pkgs.stdenv.mkDerivation {
        pname = "bouldy-docs";
        version = "0.1.0";
        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.maybeMissing ./docs;
        };
        nativeBuildInputs = [pkgs.mdbook];
        buildPhase = ''
          mdbook build docs
        '';
        installPhase = ''
          cp -r docs/book $out
        '';
      };

      site = pkgs.runCommand "bouldy-site" {} ''
        mkdir -p $out
        cp -r ${website}/* $out/
        mkdir -p $out/docs
        cp -r ${docs}/* $out/docs/
      '';
    in {
      packages = {
        inherit website docs site;
        default = site;
      };

      devShells = rs-harbor.lib.mkDevShells {
        inherit pkgs cross cargoConfig;
        inherit (toolchain) craneLib;
        packages = with pkgs; [just zola mdbook];
        extraShellHook = ''
          echo "Bouldy dev shell"
          echo "Native:  cargo check --workspace"
          echo "Windows: cargo build -p example-mod --release --target x86_64-pc-windows-gnu"
          echo "Website: cd website && zola serve"
          echo "Documentation: cd docs && mdbook serve"
        '';
      };
    });
}
