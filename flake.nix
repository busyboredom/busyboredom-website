{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk.url = "github:nix-community/naersk";
  };
  outputs = {
    self,
    nixpkgs,
    naersk,
    flake-utils,
  }:
  #with lib;
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {inherit system;};
        fmtr = nixpkgs.legacyPackages.${system}.alejandra;
        naersk' = pkgs.callPackage naersk {};
        deps = with pkgs; [
          wasm-pack
        ];
      in {
        formatter = fmtr;

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [rustup] ++ deps;
        };

        packages.default = naersk'.buildPackage {
          name = "busyboredom-website";
          version = "0.1.1";
          buildInputs = deps;
          src = ./.;

          singleStep = true;
        };

        nixosModules.default = {
          config,
          lib,
          pkgs,
        }: let
          cfg = config.services.busyboredom-website;

          configFile = with cfg pkgs;
            pkgs.writeText "config.toml" ''
              data-dir=${dataDir}
            '';
        in {
          options = with lib; {
            services.busyboredom-website = {
              enable = mkEnableOption (lib.mdDoc "Personal portfolio website");

              dataDir = lib.mkOption {
                type = types.str;
                default = "/var/lib/busyboredom-website";
                description = lib.mdDoc ''
                  The directory where busyboredom-website stores its data files.
                '';
              };

              secretsFile = lib.mkOption {
                type = types.str;
                default = "/etc/busyboredom-website/secrets.toml";
                description = lib.mdDoc ''
                  Path to file containing secrets for busyboredom-website.
                '';
              };
            };
          };

          config = lib.mkIf cfg.enable {
            users.users.busyboredomweb = {
              isSystemUser = true;
              group = "busyboredomweb";
              description = "Busyboredom.com website owner";
              home = cfg.dataDir;
              createHome = true;
            };

            users.groups.busyboredomweb = {};

            systemd.services.busyboredomweb = {
              description = "Personal portfolio website";
              after = ["network.target" "monero.service"];
              wantedBy = ["multi-user.target"];

              serviceConfig = {
                User = "busyboredomweb";
                Group = "busyboredomweb";
                ExecStart = "${self.packages.default}/bin/busyboredom --config-file=${configFile} --secrets-file=${cfg.secretsFile}";
                Restart = "always";
                SuccessExitStatus = [0 1];
              };
            };
          };
        };
      }
    );
}
