# confirmed to work on nixos with wayland (sway)
# use with `nix develop`
# then run `cargo run --example music -F winit/wayland`
{
  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };
  outputs =
    { nixpkgs, fenix, ... }:
    let
      forAllSystems =
        function:
        # insert more systems here
        nixpkgs.lib.genAttrs [ "x86_64-linux" ] (
          system:
          function (
            import nixpkgs {
              inherit system;
              overlays = [ fenix.overlays.default ];
            }
          )
        );

    in
    {
      devShells = forAllSystems (pkgs: {
        default = (pkgs.mkShell.override { stdenv = pkgs.useMoldLinker pkgs.clangStdenv; }) {
          packages = with pkgs; [
            # rust stuff
            (with pkgs.fenix; with complete; combine [
            #(with pkgs.fenix; with stable; combine [
              cargo
              clippy
              rust-src
              rustc
              rustfmt
               miri
            ])
            rust-analyzer-nightly # optional

            # necessary to build
            pkg-config # locate C dependencies
            cargo-flamegraph # more profiling :)
            cargo-nextest
            cargo-watch
            cargo-llvm-lines
            cargo-machete
            cargo-expand
            cargo-criterion
          ];
        };
      });
    };
}
