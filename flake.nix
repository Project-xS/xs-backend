{
  description = "Proj-XS";

  inputs = {
      nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
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

            testEnv = {
              S3_ENDPOINT = "http://localhost:9000";
              S3_REGION = "us-east-1";
              S3_ACCESS_KEY_ID = "test-access-key";
              S3_SECRET_KEY = "test-secret-key";
              S3_BUCKET_NAME = "test-bucket";
              AWS_EC2_METADATA_DISABLED = "true";
              DEV_BYPASS_TOKEN = "test-bypass-token";
              FIREBASE_PROJECT_ID = "test-project";
              ADMIN_JWT_SECRET = "test-admin-secret";
              DELIVER_QR_HASH_SECRET = "test-qr-secret";
            } // lib.optionalAttrs (builtins.getEnv "DATABASE_URL" != "") {
              DATABASE_URL = builtins.getEnv "DATABASE_URL";
            };

            tests = craneLib.cargoTest (
              commonArgs
              // {
                inherit cargoArtifacts;
                env = testEnv;
                cargoTestExtraArgs = "--features test-bypass -- --test-threads=1";
              }
            );

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
              tests = tests;
            };
            checks = {
              tests = tests;
            };
            defaultPackage = dockerImage;
          });
}
