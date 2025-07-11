{
  pkgs ? import <nixpkgs> { },
}:

let
  rust_overlay = import (
    builtins.fetchTarball {
      url = "https://github.com/oxalica/rust-overlay/archive/aefb7017d710f150970299685e8d8b549d653649.tar.gz";
      sha256 = "sha256:0bwxwmbg3jnyiadn6bjk6sx2as0l9slzvp0xkx16jjr8bl8z0sz7";
    }
  );
  pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
  rust = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
    extensions = [
      "rust-src"
      "rust-analyzer"
    ];
  };
in
pkgs.mkShell rec {
  libraries = with pkgs; [
    udev alsa-lib-with-plugins vulkan-loader
    xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr # To use the x11 feature
    libxkbcommon wayland # To use the wayland feature
  ];

  buildInputs =
    [
      rust
    ]
    ++ libraries
    ++ (with pkgs; [
      pkg-config
      sccache

      (gap.overrideAttrs (o: {
        version = "4.13.1";
        patches = [ ];
        src = fetchurl {
          url = "https://github.com/gap-system/gap/releases/download/v4.13.1/gap-4.13.1.tar.gz";
          sha256 = "sha256-l5Tb26b7mY4KLQqoziH8iEitPT+cyZk7C44gvn4dvro=";
        };
      }))

      (python3.withPackages (
        p: with p; [
          sympy
        ]
      ))
      python312Packages.python-lsp-server
    ]);

  RUST_BACKTRACE = 1;
  RUSTC_WRAPPER = "sccache";
  SCCACHE_SERVER_PORT = "54226";

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
}
