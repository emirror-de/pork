{
  description = "Process orchestrator with communication via UNIX/local sockets";

  inputs = {
    nixpkgs = {
      url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    {
      nixpkgs,
      fenix,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config = {
            allowUnfree = true;
          };
        };
        inherit (pkgs) lib;

        # Use the workspace's stable Rust 1.98 toolchain via fenix.
        rustToolchain =
          let
            fp = fenix.packages.${system};
          in
          fp.combine [
            fp.stable.toolchain
            fp.stable.rust-analyzer
          ];

        src = lib.cleanSourceWith {
          src = ./.;
          filter =
            path: type:
            (lib.hasSuffix ".rs" path)
            || (lib.hasSuffix ".toml" path)
            || (lib.hasSuffix ".lock" path)
            || (lib.hasSuffix ".md" path)
            || (lib.hasSuffix ".nix" path)
            || (type == "directory");
        };

        toolchainEnv = {
          inherit src;
          nativeBuildInputs = [
            rustToolchain
            pkgs.pkg-config
          ];
        };

        pork = pkgs.stdenv.mkDerivation {
          pname = "pork";
          name = "pork";
          src = src;
          buildCommand = ''
            mkdir -p $out
            cp ${./README.md} $out/README.md
            cp ${./CHANGELOG.md} $out/CHANGELOG.md
            cp ${./LICENSE-MIT} $out/LICENSE-MIT
            cp ${./LICENSE-APACHE} $out/LICENSE-APACHE
          '';
        };

        cargoFmtCheck = pkgs.runCommand "cargo-fmt-check" toolchainEnv ''
          export HOME=$TMPDIR
          cargo fmt --all --check
          touch $out
        '';

        cargoClippyCheck = pkgs.runCommand "cargo-clippy-check" toolchainEnv ''
          export HOME=$TMPDIR
          cargo clippy --workspace --all-targets --all-features -- -D warnings
          touch $out
        '';

        cargoTestCheck = pkgs.runCommand "cargo-test-check" toolchainEnv ''
          export HOME=$TMPDIR
          cargo test --workspace
          touch $out
        '';

        cargoTestAllFeaturesCheck = pkgs.runCommand "cargo-test-all-features-check" toolchainEnv ''
          export HOME=$TMPDIR
          cargo test --workspace --all-features --all-targets
          touch $out
        '';
      in
      {
        checks = {
          inherit
            pork
            cargoFmtCheck
            cargoClippyCheck
            cargoTestCheck
            cargoTestAllFeaturesCheck
            ;
        };

        packages = {
          default = pork;
        };
        # LLVM coverage removed (crane-based tooling no longer present)

        # No apps needed for a library crate

        devShells.default = pkgs.mkShell {
          name = "pork-dev";

          buildInputs = [
            rustToolchain
          ]
          ++ (with pkgs; [
            pkg-config
            taplo
            cargo-audit
            cargo-limit
            cargo-deny
            cargo-nextest
            cargo-watch
            cargo-expand
            cargo-machete
            cargo-leptos
            cargo-sort
          ])
          ++ lib.optionals (pkgs.stdenv.isLinux) [
          ];

          shellHook = "";
        };

        # Formatter for the flake itself
        formatter = pkgs.nixfmt;
      }
    );
}
