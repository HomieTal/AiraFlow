# Home-manager module for Aira Flow speech-to-text
#
# Provides a systemd user service for autostart.
# Usage: imports = [ aira-flow.homeManagerModules.default ];
#        services.aira-flow.enable = true;
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.aira-flow;
in
{
  options.services.aira-flow = {
    enable = lib.mkEnableOption "Aira Flow speech-to-text user service";

    package = lib.mkOption {
      type = lib.types.package;
      defaultText = lib.literalExpression "aira-flow.packages.\${system}.aira-flow";
      description = "The Aira Flow package to use.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services.aira-flow = {
      Unit = {
        Description = "Aira Flow speech-to-text";
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${cfg.package}/bin/aira-flow";
        Restart = "on-failure";
        RestartSec = 5;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
