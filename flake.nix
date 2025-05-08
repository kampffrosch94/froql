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
            (
              #with pkgs.fenix; with complete; combine [
              with pkgs.fenix; with stable; combine [
                cargo
                clippy
                rust-src
                rustc
                rustfmt
                #miri # needs complete/nightly
              ]
            )
            rust-analyzer-nightly # optional

            cargo-flamegraph # more profiling :)
            cargo-nextest
            cargo-watch
            cargo-llvm-lines
            cargo-machete
            cargo-expand
            cargo-criterion
            cargo-insta
            mdbook

            ### begin macroquad
            # necessary to build
            pkg-config # locate C dependencies
            alsa-lib # sound
            libxkbcommon # keyboard

            # X
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr

            # gl
            libGL
          ];
          LD_LIBRARY_PATH =
            with pkgs;
            lib.makeLibraryPath [
              alsa-lib # sound
              libGL
              libxkbcommon # keyboard
              xorg.libX11
              xorg.libXi
            ];
          env.LIBCLANG_PATH = "${pkgs.llvmPackages.clang-unwrapped.lib}/lib/libclang.so";
          ### end macroquad
        };
      });
    };
}
