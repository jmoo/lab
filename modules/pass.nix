{
  pkgs,
  config,
  lib,
  ...
}:
with lib;
{
  options.lab.pass = {
    enable = mkEnableOption "Enable password-store home-manager configuration";

    user = mkOption {
      description = "User to use for gpg key and password store";
      type = types.str;
      default = "me@jmoo.io";
    };
  };

  config = mkIf config.lab.pass.enable {
    programs.gpg.enable = true;
    programs.password-store.enable = true;

    services.gpg-agent = {
      enable = true;
      pinentryPackage = pkgs.pinentry-curses;
    };

    home.activation = {
      reload-gpg-agent = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        run ${config.programs.gpg.package}/bin/gpgconf --reload gpg-agent
      '';

      init-password-store = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        run ${config.programs.password-store.package}/bin/pass init ${config.lab.pass.user}
        run cd ~/.local/share/password-store && ${pkgs.git}/bin/git init
        run cd ~/.local/share/password-store && ${pkgs.git}/bin/git config user.name "Password Store"
        run cd ~/.local/share/password-store && ${pkgs.git}/bin/git config user.email ${config.lab.pass.user}
        run ${config.programs.password-store.package}/bin/pass git init
        run ${config.programs.password-store.package}/bin/pass init ${config.lab.pass.user}
      '';
    };
  };
}
