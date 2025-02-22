{
  pkgs ? import <nixpkgs> { },
}:

let
  rust_overlay = import (
    builtins.fetchTarball {
      url = "https://github.com/oxalica/rust-overlay/archive/83284068670d5ae4a43641c4afb150f3446be70d.tar.gz";
      sha256 = "sha256:0z5cym494khqy3pxfwfq89nb2981v8q8wb4kxn04i6qj34gjp8ab";
    }
  );
  pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
  rust = pkgs.rust-bin.nightly."2025-02-22".default.override {
    extensions = [ "rust-src" "rust-analyzer" ];
  };
in
pkgs.mkShell {
  buildInputs =
    [
      rust
    ]
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
}
