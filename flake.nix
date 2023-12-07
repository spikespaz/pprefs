{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixfmt.url = "github:serokell/nixfmt";
  };

  outputs = { self, nixpkgs, systems, rust-overlay, nixfmt }:
    let
      inherit (nixpkgs) lib;
      eachSystem = lib.genAttrs (import systems);
      pkgsFor = eachSystem (system:
        import nixpkgs {
          localSystem = system;
          overlays = [ rust-overlay.overlays.default ];
        });
    in {
      devShells = eachSystem (system:
        let pkgs = pkgsFor.${system};
        in {
          default = pkgs.mkShell {
            strictDeps = true;

            packages = with pkgs; [
              (lib.hiPrio (rust-bin.stable.latest.minimal.override {
                extensions = [ "rust-src" "rust-docs" "clippy" ];
              }))
              (rust-bin.selectLatestNightlyWith (toolchain:
                toolchain.minimal.override {
                  extensions = [ "rustfmt" "rust-analyzer" ];
                }))
            ];

            # RUST_BACKTRACE = 1;
          };
        });

      formatter = eachSystem (system: nixfmt.packages.${system}.default);
    };
}
