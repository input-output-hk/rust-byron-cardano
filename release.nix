{ system ? builtins.currentSystem, config ? {} }:
let
  iohkNix = import (
    let try = builtins.tryEval <iohk_nix>;
    in if try.success
    then builtins.trace "using host <iohk_nix>" try.value
    else
      let
        spec = builtins.fromJSON (builtins.readFile ./iohk-nix.json);
      in builtins.fetchTarball {
        url = "${spec.url}/archive/${spec.rev}.tar.gz";
        inherit (spec) sha256;
      }) { inherit config system; };
  pkgs = iohkNix.rust-packages.pkgs;
  src = pkgs.lib.cleanSource ./.;

in {
  docs = {
    cardano-c = pkgs.runCommand "cardano-c-docs" { buildInputs = [ pkgs.doxygen ]; } ''
        mkdir -p cardano-c/docs
        cp -a ${src}/cardano-c/* cardano-c/
        cd cardano-c
        ls
        doxygen
        mkdir -p $out/nix-support
        cp -a docs/* $out/
        echo "doc manual $out/html index.html" > $out/nix-support/hydra-build-products
      '';
  };
}
