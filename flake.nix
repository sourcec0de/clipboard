{
  description = "clipboard — zero-network, cross-platform Native Shell (nsh) clipboard manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        buildInputs =
          with pkgs;
          lib.optionals stdenv.hostPlatform.isLinux [
            gtk4
            pango
            glib
            cairo
            gdk-pixbuf
            graphene
          ]
          ++ lib.optionals stdenv.hostPlatform.isDarwin [
            libiconv
          ];

        clipboard = rustPlatform.buildRustPackage {
          pname = "clipboard";
          version = "0.1.0";
          src = self;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          cargoBuildFlags = [ "--workspace" ];
          cargoTestFlags = [ "--workspace" ];

          meta = with pkgs.lib; {
            description = "Zero-network cross-platform clipboard manager built with Native Shell";
            mainProgram = "clipboard-app";
            platforms = platforms.unix ++ platforms.windows;
          };
        };
      in
      {
        packages = {
          default = clipboard;
          inherit clipboard;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = clipboard;
          name = "clipboard-app";
        };

        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;
          packages = with pkgs; [
            rustToolchain
            git
          ];

          env = {
            RUST_BACKTRACE = "1";
          }
          // pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          };

          shellHook = ''
            echo "clipboard · $(rustc --version)"
          '';
        };

        checks = {
          package = clipboard;

          fmt = pkgs.runCommand "clipboard-fmt" { nativeBuildInputs = [ rustToolchain ]; } ''
            cd ${self}
            cargo fmt --all -- --check
            touch $out
          '';

          clippy = clipboard.overrideAttrs (old: {
            pname = "clipboard-clippy";
            doCheck = false;
            buildPhase = ''
              runHook preBuild
              cargo clippy --workspace --all-targets --offline -- -D warnings
              runHook postBuild
            '';
            installPhase = ''
              touch $out
            '';
          });
        };
      }
    );
}
