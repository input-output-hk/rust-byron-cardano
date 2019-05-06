{ nixpkgs ? fetchTarball https://github.com/NixOS/nixpkgs/archive/ad1e17f5ba27ace0f5e5529b649272ac83da8e65.tar.gz
, pkgs ? import nixpkgs {}
}:

with pkgs;

stdenv.mkDerivation {
  name = "rust-cardano";

  src = null;

  buildInputs = [ rustc cargo sqlite protobuf rustfmt ];

  # FIXME: we can remove this once prost is updated.
  PROTOC = "${protobuf}/bin/protoc";
}
