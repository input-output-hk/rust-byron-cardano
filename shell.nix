{ nixpkgs ? fetchTarball https://github.com/NixOS/nixpkgs/archive/7fff567ee99c1f343ecdd82fef2e35fb6f50e423.tar.gz
, pkgs ? import nixpkgs {}
}:

with pkgs;

stdenv.mkDerivation {
  name = "rust-carano";

  src = null;

  buildInputs = [ rustc cargo sqlite protobuf rustfmt ];

  # FIXME: we can remove this once prost is updated.
  PROTOC = "${protobuf}/bin/protoc";
}
