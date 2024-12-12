{ pkgs ? import <nixpkgs> {}, system ? builtins.currentSystem }:

let
  src = fetchTarball "https://github.com/numtide/devshell/archive/main.tar.gz";
  devshell = import src { inherit system; };

  rust_overlay = import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
  rust = pkgs.rust-bin.stable."1.81.0".default.override {
    extensions = [ "rust-src" ];
  };
in
devshell.mkShell {
  packages = [
    rust
  ] ++ (with pkgs; [
    pkg-config
    rust-analyzer
    sccache

    clang

    (gap.overrideAttrs (o: {
      version = "4.13.1";
      patches = [ ];
      src = fetchurl {
        url = "https://github.com/gap-system/gap/releases/download/v4.13.1/gap-4.13.1.tar.gz";
        sha256 = "sha256-l5Tb26b7mY4KLQqoziH8iEitPT+cyZk7C44gvn4dvro=";
      };
    }))

    python3
    python312Packages.python-lsp-server
  ]);

  env = [
    {
      name = "RUST_BACKTRACE";
      value = 1;
    }
    {
      name = "RUSTC_WRAPPER";
      value = "sccache";
    }
    {
      name = "SCCACHE_SERVER_PORT";
      value = "54226";
    }
    # RUST_BACKTRACE = 1;
    # RUSTC_WRAPPER = "sccache";
    # SCCACHE_SERVER_PORT = "54226";
  ];
}


