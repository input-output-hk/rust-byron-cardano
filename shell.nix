{ nixpkgs ? fetchTarball https://github.com/NixOS/nixpkgs/archive/8d3e91077ba074e2c947a152ee8ab7be885c42ab.tar.gz
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
