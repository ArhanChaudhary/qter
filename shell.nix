{
  pkgs ? import <nixpkgs> { },
}:

let
  rust_overlay = import (
    builtins.fetchTarball {
      url = "https://github.com/oxalica/rust-overlay/archive/74a3fb71b0cc67376ab9e7c31abcd68c813fc226.tar.gz";
      sha256 = "sha256:1y308q8yxz8nkdw2gh626ay8am8c5w4jn0kfc37wiiqza0xp392s";
    }
  );
  pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
  rust = pkgs.rust-bin.nightly."2025-03-02".default.override {
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
