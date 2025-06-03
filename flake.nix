{
  description = "Cheater's Swear Jar rust (and nix) flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {inherit system overlays;};
    in {
      devShells.default = with pkgs;
        mkShell {
          buildInputs = [openssl];
          nativeBuildInputs = [
            cargo-shuttle
            nil
            nixd
            alejandra
            pkg-config
            (rust-bin.stable.latest.default.override {
              extensions = ["rust-analyzer" "rust-src"];
            })
          ];
        };
      packages.default = with pkgs; let
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      in
        rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          version = cargoToml.package.version;
          meta.mainProgram = cargoToml.package.name;
          buildInputs = [openssl];
          nativeBuildInputs = [pkg-config];
          src = lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };
    });
}
