{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeDarwin;
  inherit (lib')
    mkEnableOption
    mkIf
    mkOption
    types
    ;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    let
      cfg = config.ollama;
    in
    {
      options.ollama = {
        enable = mkEnableOption "ollama server home-manager configuration (darwin)";

        models = mkOption {
          default = [ ];
          description = "Models pulled in the background whenever the agent loads.";
          example = [ "qwen3.8:27b-q8_0" ];
          type = types.listOf types.str;
        };
      };

      config = mkIf cfg.enable (
        homeDarwin (
          { config, pkgs, ... }:
          let
            ollama = config.services.ollama;
            pull = pkgs.writeShellApplication {
              name = "ollama-pull";
              runtimeInputs = [ ollama.package ];
              text = ''
                until ollama list > /dev/null 2>&1; do sleep 2; done
                for model in "$@"; do
                  ollama pull "$model"
                done
              '';
            };
          in
          {
            launchd.agents.ollama-pull = mkIf (cfg.models != [ ]) {
              config = {
                EnvironmentVariables.OLLAMA_HOST = "${ollama.host}:${toString ollama.port}";
                ProgramArguments = [ "${pull}/bin/ollama-pull" ] ++ cfg.models;
                RunAtLoad = true;
                StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/ollama-pull.log";
                StandardOutPath = "${config.home.homeDirectory}/Library/Logs/ollama-pull.log";
              };
              enable = true;
            };

            services.ollama.enable = true;
          }
        )
      );
    }
  );
}
