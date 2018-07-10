{ config, lib, pkgs, ... }:

with lib;

let

  build = import ./. { /* inherit pkgs; */ };

  cfg = config.services.hermes;

in

{

  options = {

    services.hermes = {

      port = mkOption {
        type = types.int;
        default = 3080;
        description = "The TCP port on which Hermes listens for HTTP connections.";
      };

    };

  };

  config = {

    environment.systemPackages = [ build ];

    users.users.hermes = {
      description = "Hermes server user";
      isSystemUser = true;
      group = "hermes";
    };

    users.groups.hermes = {};

    system.activationScripts.hermes = stringAfter [ "users" ]
      ''
        install -d -m 0755 -o hermes -g hermes /var/lib/hermes
      '';

    systemd.services.hermes = {
      description = "Hermes Web Service";
      after = [ "syslog.target" "network.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        User = "hermes";
        ExecStart = "${build}/bin/hermes start --port ${toString cfg.port} --networks-dir /var/lib/hermes/networks";
      };
    };

  };

}
