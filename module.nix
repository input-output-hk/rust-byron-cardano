{ config, lib, pkgs, ... }:

with lib;

let

  build = import ./. { /* inherit pkgs; */ };

in

{

  options = {

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
        ExecStart = "${build}/bin/hermes start --port 3080 --networks-dir /var/lib/hermes/networks";
      };
    };

  };

}
