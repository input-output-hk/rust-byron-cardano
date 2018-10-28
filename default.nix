{ nixpkgs ? <nixpkgs>
, pkgs ? import nixpkgs {}
}:
with pkgs;
let
  inherit (pkgs) lib buildPlatform buildRustCrate buildRustCrateHelpers fetchgit;
  cratesIO = callPackage ./crates-io.nix { inherit buildRustCrateHelpers; };
in
(import ./Cargo.nix { inherit lib buildPlatform buildRustCrate buildRustCrateHelpers cratesIO fetchgit; })
