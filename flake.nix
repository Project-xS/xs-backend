{
  description = "Proj-XS";

  inputs = {
      nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
      flake-utils.url = "github:numtide/flake-utils";
      crane.url = "github:ipetkov/crane";
    };

    outputs = { self, nixpkgs, flake-utils, crane, ... }:
        flake-utils.lib.eachDefaultSystem (system:
          let
            pkgs = nixpkgs.legacyPackages.${system};

            inherit (pkgs) lib;

            craneLib = crane.mkLib pkgs;
            unfilteredRoot = ./.;
            src = lib.fileset.toSource {
              root = unfilteredRoot;
              fileset = lib.fileset.unions [
                (craneLib.fileset.commonCargoSources unfilteredRoot)
                ./migrations
              ];
            };

            commonArgs = {
              inherit src;
              strictDeps = true;

              nativeBuildInputs = with pkgs; [
                pkg-config
                perl
                openssl
                openssl.dev
                curl
              ];
            };

            cargoArtifacts = craneLib.buildDepsOnly commonArgs;

            my-rust-build = craneLib.buildPackage (
              commonArgs
              // {
                inherit cargoArtifacts;

                nativeBuildInputs = (commonArgs.nativeBuildInputs or [ ]);
              }
            );

            dockerImage = pkgs.dockerTools.buildLayeredImage {
              name = "proj-xs";
              tag = "latest";
              config = {
                Env = [ "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt" ];
                Cmd = [ "${my-rust-build}/bin/proj-xs" ];
              };
            };

          in {
            packages = {
              docker = dockerImage;
            };
            defaultPackage = dockerImage;
          });
}
