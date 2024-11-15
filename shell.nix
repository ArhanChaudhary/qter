{ pkgs ? import <nixpkgs> {} }:

let
  rust_overlay = import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
  rust = pkgs.rust-bin.stable."1.81.0".default.override {
    extensions = [ "rust-src" ];
  };
in
pkgs.mkShell {
  buildInputs = [
    rust
  ] ++ (with pkgs; [
    pkg-config
    rust-analyzer
    sccache

    gap
    
    python3
    python312Packages.python-lsp-server
  ]);

  RUST_BACKTRACE = 1;
  RUSTC_WRAPPER = "sccache";
}


