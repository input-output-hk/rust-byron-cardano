let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
in
  with nixpkgs;
  stdenv.mkDerivation {
    name = "moz_overlay_shell";
    buildInputs = [
      (nixpkgs.latest.rustChannels.nightly.rust.override { targets = ["wasm32-unknown-unknown"]; })
      nodejs
      nixpkgs.nodePackages.webpack
    ];
    shellHook = ''
      export PATH=$PATH:$(pwd)/node_modules/
    '';
  }
