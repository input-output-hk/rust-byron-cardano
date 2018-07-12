{ nixpkgs ? fetchTarball channel:nixos-unstable
, pkgs ? import nixpkgs {}
}:

with pkgs;

rustPlatform.buildRustPackage {
  name = "rust-cardano";

  src = fetchGit ./.;

  cargoSha256 = "1spxgxh6xbhn7828a30hd74dxwc7j3m7y1isb15n6zm4jxrvj6wx";
}
