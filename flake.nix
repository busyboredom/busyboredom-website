{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    crane.url = "github:ipetkov/crane";
  };
  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        fmtr = pkgs.nixpkgs-fmt;
        craneLib = crane.mkLib pkgs;
        deps = with pkgs; [
          wasm-pack
        ];
      in
      {
        formatter = fmtr;

        devShells.default = pkgs.mkShell {
          packages =
            with pkgs;
            [
              rustup

              # Needed for some tools like `cargo-outdated`.
              pkg-config
              openssl
            ]
            ++ deps;
        };

        packages.default = craneLib.buildPackage {
          name = "busyboredom";
          version = "0.1.1";
          buildInputs = deps;
          src = ./.;
          RUST_MIN_STACK = "33554434";
          singleStep = true;
        };

        nixosModule =
          {
            config,
            lib,
            pkgs,
            ...
          }:
          let
            package = self.packages.${system}.default;

            cfg = config.services.busyboredom-website;
            configFile =
              with cfg;
              pkgs.writeText "config.toml" ''
                data_dir="${dataDir}"
              '';
          in
          {
            options = with lib; {
              services.busyboredom-website = {
                enable = mkEnableOption (lib.mdDoc "Personal portfolio website");

                dataDir = mkOption {
                  type = types.str;
                  default = "/var/lib/busyboredom-website";
                  description = lib.mdDoc ''
                    The directory where busyboredom-website stores its data files.
                  '';
                };

                emailPasswordFile = mkOption {
                  type = types.str;
                  default = "${cfg.dataDir}/email_password.txt";
                  description = lib.mdDoc ''
                    Path to file containing email password for busyboredom-website.
                  '';
                };

                daemonPasswordFile = mkOption {
                  type = types.str;
                  default = "${cfg.dataDir}/daemon_password.txt";
                  description = lib.mdDoc ''
                    Path to file containing daemon password for busyboredom-website.
                  '';
                };

                privateViewkeyFile = mkOption {
                  type = types.str;
                  default = "${cfg.dataDir}/private_viewkey.txt";
                  description = lib.mdDoc ''
                    Path to private viewkey for busyboredom-website.
                  '';
                };

                waitFor = mkOption {
                  type = with types; listOf str;
                  default = [ ];
                  description = lib.mdDoc ''
                    List of systemd services to wait for.
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

              users.groups.busyboredomweb = { };

              systemd.services.busyboredomweb = {
                description = "Personal portfolio website";
                after = [ "network.target" ] ++ cfg.waitFor;
                wants = [ "network.target" ] ++ cfg.waitFor;
                wantedBy = [ "multi-user.target" ];
                script = ''
                  export EMAIL_PASSWORD=$(cat ${cfg.emailPasswordFile})
                  export DAEMON_PASSWORD=$(cat ${cfg.daemonPasswordFile})
                  export XMR_PRIVATE_VIEWKEY=$(cat ${cfg.privateViewkeyFile})
                  ${package}/bin/busyboredom --config-file=${configFile}
                '';

                serviceConfig = {
                  User = "busyboredomweb";
                  Group = "busyboredomweb";
                  Restart = "always";
                  SuccessExitStatus = [
                    0
                    1
                  ];
                };
              };
            };
          };
      }
    );
}
