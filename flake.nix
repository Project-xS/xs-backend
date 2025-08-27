{
  description = "Proj-XS";

  inputs = {
      nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
      flake-utils.url = "github:numtide/flake-utils";
      rust-overlay.url = "github:oxalica/rust-overlay";
    };

    outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
        flake-utils.lib.eachDefaultSystem (system:
          let
            overlays = [ (import rust-overlay) ];
            pkgs = import nixpkgs { inherit system overlays; };
            rustVersion = pkgs.rust-bin.stable.latest.default;

            swaggerUiSrc = pkgs.fetchurl {
              url = "https://github.com/swagger-api/swagger-ui/archive/refs/tags/v5.17.14.zip";
              sha256 = "sha256-SBJE0IEgl7Efuu73n3HZQrFxYX+cn5UU5jrL4T5xzNw=";
            };

            rustPlatform = pkgs.makeRustPlatform {
              cargo = rustVersion;
              rustc = rustVersion;
            };

            myRustBuild = rustPlatform.buildRustPackage {
              pname =
                "proj-xs";
              version = "0.1.0";
              src = ./.;

              preBuild = ''
                cp "${swaggerUiSrc}" ./swagger.zip
                chmod 666 ./swagger.zip
                export SWAGGER_UI_DOWNLOAD_URL="file://$(realpath ./swagger.zip)"
              '';

              cargoLock.lockFile = ./Cargo.lock;
              nativeBuildInputs = [ pkgs.pkg-config pkgs.perl pkgs.git pkgs.openssl pkgs.openssl.dev pkgs.curl ];

            };

            dockerImage = pkgs.dockerTools.buildLayeredImage {
              name = "proj-xs";
              tag = "latest";
              config = {
                Env = [ "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt" ];
                Cmd = [ "${myRustBuild}/bin/proj-xs" ];
              };
            };

          in {
            packages = {
              rustPackage = myRustBuild;
              docker = dockerImage;
            };
            defaultPackage = dockerImage;
          });
}
