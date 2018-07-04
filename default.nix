{ nixpkgs ? fetchTarball channel:nixos-unstable }:

with import nixpkgs {};

rustPlatform.buildRustPackage {
  name = "rust-cardano";

  src = fetchGit ./.;

  cargoSha256 = "0d910by4siv5glw0j89mrj3ysn16w9g9kq1wxzrr46wm053m2nyl";
}
