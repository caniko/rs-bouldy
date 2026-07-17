{
  description = "Bouldy: Rust runtime modding MVP for Unreal Engine games";

  # Advertise the private macOS SDK Attic cache so darwin cross-compiles
  # substitute the realized SDK from the pin instead of rebuilding it.
  nixConfig = {
    extra-substituters = ["https://attic.candee.baby/harbor-macos-sdk"];
    extra-trusted-public-keys = [
      "harbor-macos-sdk:ci7MNMkHDqdeTS4aKwzDNEJ1175AbpVUypTRjCJoHDk="
    ];
  };

  inputs = {
    rs-harbor.url = "git+https://codeberg.org/caniko/rs-harbor.git?ref=trunk";

    rs-harbor-macos-sdk-pin.url = "git+ssh://git@codeberg.org/caniko/rs-harbor-macos-sdk-pin.git";

    nixpkgs.follows = "rs-harbor/nixpkgs";
    rust-overlay.follows = "rs-harbor/rust-overlay";
    crane.follows = "rs-harbor/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rs-harbor,
    rs-harbor-macos-sdk-pin,
    flake-utils,
    rust-overlay,
    ...
  }: let
    mkOutputs = {
      macosSdkStorePath ? rs-harbor-macos-sdk-pin.storePath,
      osxSdkVersion ? rs-harbor-macos-sdk-pin.sdkVersion,
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

        cross = rs-harbor.lib.mkCross ({
            inherit pkgs system osxSdkVersion;
          }
          // lib.optionalAttrs (macosSdkStorePath != null) {
            inherit macosSdkStorePath;
          });

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

        mkUe4ssPackage = {
          pname,
          version,
          url,
          hash,
        }:
          pkgs.stdenvNoCC.mkDerivation {
            inherit pname version;

            src = pkgs.fetchurl {
              inherit url hash;
            };

            nativeBuildInputs = [pkgs.unzip];

            dontUnpack = true;
            dontFixup = true;
            dontStrip = true;

            installPhase = ''
              runHook preInstall

              mkdir -p "$out/share/ue4ss"
              unzip -q "$src" -d "$out/share/ue4ss"

              runHook postInstall
            '';

            meta = {
              description = "UE4SS Unreal Engine scripting system release archive";
              homepage = "https://github.com/UE4SS-RE/RE-UE4SS";
              license = lib.licenses.mit;
              platforms = lib.platforms.all;
            };
          };

        ue4ss = mkUe4ssPackage {
          pname = "ue4ss";
          version = "3.0.1";
          url = "https://github.com/UE4SS-RE/RE-UE4SS/releases/download/v3.0.1/UE4SS_v3.0.1.zip";
          hash = "sha256-S0fUvO3dL1YaTjlb+gCSTM/JRa9Xai0MYT5lN4RsV+w=";
        };

        ue4ss-dev = mkUe4ssPackage {
          pname = "ue4ss-dev";
          version = "3.0.1";
          url = "https://github.com/UE4SS-RE/RE-UE4SS/releases/download/v3.0.1/zDEV-UE4SS_v3.0.1.zip";
          hash = "sha256-r3d9KvM/zk1SDeNahmXkFzdrzFMP7GR3eAEs5s31Gys=";
        };

        mkInstallUe4ssApp = name: ue4ssPackage: let
          script = pkgs.writeShellApplication {
            inherit name;
            runtimeInputs = [
              pkgs.coreutils
              pkgs.findutils
              pkgs.gnugrep
            ];
            text = ''
              usage() {
                cat <<'USAGE'
              Usage:
                install-ue4ss --win64-dir /path/to/Game/Binaries/Win64

              Installs the pinned UE4SS release into a game's Win64 executable directory.
              USAGE
              }

              win64_dir=""

              while [ "$#" -gt 0 ]; do
                case "$1" in
                  --win64-dir)
                    if [ "$#" -lt 2 ]; then
                      echo "error: --win64-dir requires a path" >&2
                      usage >&2
                      exit 2
                    fi
                    win64_dir="$2"
                    shift 2
                    ;;
                  -h|--help)
                    usage
                    exit 0
                    ;;
                  *)
                    echo "error: unknown argument: $1" >&2
                    usage >&2
                    exit 2
                    ;;
                esac
              done

              if [ -z "$win64_dir" ]; then
                echo "error: expected --win64-dir" >&2
                usage >&2
                exit 2
              fi

              ensure_bouldy_mod_enabled() {
                mods_file="$1"
                enabled_line="StellarBladeBouldyRecon : 1"
                tmp_file="$mods_file.tmp.$$"
                inserted=0

                if [ -f "$mods_file" ]; then
                  while IFS= read -r line || [ -n "$line" ]; do
                    if printf '%s\n' "$line" | grep -Eq '^[[:space:]]*StellarBladeBouldyRecon[[:space:]]*:'; then
                      continue
                    fi

                    case "$line" in
                      "; Built-in keybinds, do not move up!"*)
                        if [ "$inserted" -eq 0 ]; then
                          printf '%s\n' "$enabled_line"
                          inserted=1
                        fi
                        printf '%s\n' "$line"
                        ;;
                      *)
                        printf '%s\n' "$line"
                        ;;
                    esac
                  done < "$mods_file" > "$tmp_file"

                  if [ "$inserted" -eq 0 ]; then
                    printf '\n%s\n' "$enabled_line" >> "$tmp_file"
                  fi
                else
                  printf '%s\n' "$enabled_line" > "$tmp_file"
                fi

                mv "$tmp_file" "$mods_file"
              }

              if [ ! -d "$win64_dir" ]; then
                echo "error: missing Win64 directory: $win64_dir" >&2
                echo "required because UE4SS must be installed beside the game's Win64 shipping executable" >&2
                echo "upstream producer: local Unreal Engine game install" >&2
                echo "regenerate/retry: pass the correct --win64-dir" >&2
                echo "validation: test -d '$win64_dir'" >&2
                exit 1
              fi

              if ! find "$win64_dir" -maxdepth 1 -name '*-Win64-Shipping.exe' -type f -print -quit | grep -q .; then
                echo "error: missing Win64 shipping executable in: $win64_dir" >&2
                echo "required because this confirms the target is an Unreal game Win64 runtime directory" >&2
                echo "upstream producer: local Unreal Engine game install" >&2
                echo "regenerate/retry: verify the game files or pass the correct --win64-dir" >&2
                echo "validation: find '$win64_dir' -maxdepth 1 -name '*-Win64-Shipping.exe' -type f -print -quit" >&2
                exit 1
              fi

              ue4ss_src="${ue4ssPackage}/share/ue4ss"
              for managed in UE4SS.dll dwmapi.dll UE4SS-settings.ini; do
                source_file="$ue4ss_src/$managed"
                target_file="$win64_dir/$managed"
                if [ ! -f "$source_file" ]; then
                  echo "error: missing pinned UE4SS artifact: $source_file" >&2
                  echo "required because $managed is a managed file in the pinned UE4SS release" >&2
                  echo "upstream producer: UE4SS-RE/RE-UE4SS release archive" >&2
                  echo "regenerate/retry: nix build ./vendor/bouldy#ue4ss" >&2
                  echo "validation: test -f '$source_file'" >&2
                  exit 1
                fi
                if [ -e "$target_file" ] && ! cmp -s "$source_file" "$target_file"; then
                  echo "error: existing managed UE4SS file differs from pinned release: $target_file" >&2
                  echo "required because overwriting an unknown loader/settings file could break an existing install" >&2
                  echo "upstream producer: UE4SS-RE/RE-UE4SS release archive or the existing local UE4SS install" >&2
                  echo "regenerate/retry: remove or back up '$target_file', then rerun this installer" >&2
                  echo "validation: cmp -s '$source_file' '$target_file'" >&2
                  exit 1
                fi
              done

              install -m 0644 "$ue4ss_src/UE4SS.dll" "$win64_dir/UE4SS.dll"
              install -m 0644 "$ue4ss_src/dwmapi.dll" "$win64_dir/dwmapi.dll"
              install -m 0644 "$ue4ss_src/UE4SS-settings.ini" "$win64_dir/UE4SS-settings.ini"
              [ -f "$ue4ss_src/README.md" ] && install -m 0644 "$ue4ss_src/README.md" "$win64_dir/README.md"
              [ -f "$ue4ss_src/Changelog.md" ] && install -m 0644 "$ue4ss_src/Changelog.md" "$win64_dir/Changelog.md"

              mods_dir="$win64_dir/Mods"
              mkdir -p "$mods_dir"
              if [ -d "$ue4ss_src/Mods" ]; then
                for item in "$ue4ss_src"/Mods/*; do
                  [ -e "$item" ] || continue
                  if [ "$(basename "$item")" = "mods.txt" ]; then
                    continue
                  fi
                  if [ -e "$mods_dir/$(basename "$item")" ]; then
                    chmod -R u+rwX "$mods_dir/$(basename "$item")"
                  fi
                  cp -R --no-preserve=mode "$item" "$mods_dir/"
                  chmod -R u+rwX "$mods_dir/$(basename "$item")"
                done
              fi

              mods_file="$mods_dir/mods.txt"
              if [ ! -f "$mods_file" ] && [ -f "$ue4ss_src/Mods/mods.txt" ]; then
                cp "$ue4ss_src/Mods/mods.txt" "$mods_file"
                chmod 0644 "$mods_file"
              fi
              ensure_bouldy_mod_enabled "$mods_file"

              if find "$win64_dir" -maxdepth 1 \
                \( -name 'dxgi.dll' -o -name 'xinput1_3.dll' -o -name 'dsound.dll' \) \
                -print -quit | grep -q .; then
                echo "warning: non-UE4SS proxy DLLs are present in $win64_dir" >&2
                echo "warning: verify loader compatibility if UE4SS does not start" >&2
              fi

              echo "installed pinned UE4SS to: $win64_dir"
              echo "updated UE4SS mods file: $mods_file"
            '';
          };
        in {
          type = "app";
          program = "${script}/bin/${name}";
        };
      in {
        packages = {
          inherit website docs site ue4ss ue4ss-dev;
          default = site;
        };

        apps = {
          install-ue4ss = mkInstallUe4ssApp "install-ue4ss" ue4ss;
          install-ue4ss-dev = mkInstallUe4ssApp "install-ue4ss-dev" ue4ss-dev;
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
  in
    mkOutputs {}
    // {
      lib = {
        inherit mkOutputs;
      };
    };
}
