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

        # Use nightly Rust toolchain and include wasm target via fenix.packages.${system}
        rustToolchain =
          let
            fp = fenix.packages.${system};
          in
          fp.combine [
            fp.latest.toolchain
            fp.latest.rust-analyzer
          ];

        src = lib.cleanSourceWith {
          src = ./.;
          filter =
            path: type:
            (lib.hasSuffix ".rs" path)
            || (lib.hasSuffix ".toml" path)
            || (lib.hasSuffix ".lock" path)
            || (lib.hasSuffix ".md" path)
            || (type == "directory");
        };

        pork = pkgs.stdenv.mkDerivation {
          pname = "pork";
          name = "pork";
          src = src;
          buildCommand = ''
            # placeholder build - no-op
            mkdir -p $out
            echo "pork placeholder" > $out/README
          '';
        };
      in
      {
        checks = {
          inherit pork;
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

          shellHook = ''
          '';
        };

        # Formatter for the flake itself
        formatter = pkgs.nixfmt;
      }
    );
}
