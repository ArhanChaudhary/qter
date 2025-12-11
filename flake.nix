{
  inputs = {
    nixpkgs.url = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rust = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
          targets = [ "aarch64-unknown-linux-gnu" ];
        };

        libraries = with pkgs; [
          udev
          alsa-lib-with-plugins
          vulkan-loader
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr # To use the x11 feature
          libxkbcommon
          wayland # To use the wayland feature
        ];
      in
      rec {
        devShell = pkgs.mkShell rec {
          buildInputs =
            libraries
            ++ (with pkgs; [
              sccache
              rust-analyzer
              rust
              pkg-config
              packages.rob-twophase

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
          RUSTFLAGS = "-C target-cpu=native";

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;

          shellHook = ''
            export PATH=$PATH:~/.cargo/bin
          '';
        };

        packages.rob-twophase = pkgs.stdenv.mkDerivation {
          name = "rob-twophase";
          src = pkgs.fetchFromGitHub {
            owner = "efrantar";
            repo = "rob-twophase";
            rev = "d245031257d52b2663c5790c5410ef30aefd775f";
            hash = "sha256-2QZgW7w80+oKlMFMkIvuEXdp0SkIXpLs02MHe9qjb/c=";
          };
          buildPhase = ''
            make
          '';
          installPhase = ''
            mkdir -p $out/bin
            cp twophase $out/bin
          '';
        };

        robot-deps = [ packages.rob-twophase ];

        legacyPackages = packages;
      }
    );
}
