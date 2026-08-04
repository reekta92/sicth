{
  description = "sicth — minimal TUI file navigator";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "sicth";
          version = "1.0.1"; # auto-updated
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          meta = with pkgs.lib; {
            description = "A minimal TUI file navigator with fuzzy search";
            homepage = "https://github.com/reekta92/sicth";
            license = licenses.gpl3Only;
            mainProgram = "sicth";
          };
        };
      });
}