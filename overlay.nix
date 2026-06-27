inputs: final: prev:
let
  lib' = prev.lib.extend (import ./lib.nix inputs);
  scriptsDir = ./scripts;
  scriptsEntries = builtins.readDir scriptsDir;
  scriptFiles = builtins.filter (name: scriptsEntries.${name} == "regular") (
    builtins.attrNames scriptsEntries
  );
in
{
  # # Fix core dump on asahi
  # # This PR gets a little farther but still segfaults
  # hyprlock = prev.hyprlock.overrideAttrs (_: {
  #   src = final.fetchFromGitHub {
  #     owner = "jaakkomoller";
  #     repo = "hyprlock";
  #     rev = "839";
  #     hash = "sha256-raYdkw32pEE9HetrIu7jHOWiSmp8YTBLMPVekV46+I4=";
  #   };
  # });

  nudelta = inputs.nudelta.packages.${prev.stdenv.hostPlatform.system}.default;

  open-bamboo-networking = final.callPackage ./pkgs/open-bamboo-networking { };

  ulauncher-uwsm = final.callPackage ./pkgs/ulauncher-uwsm { };

  vscode-extensions = prev.vscode-extensions // {
    mkVscodeNixExtension =
      config:
      final.vscode-extensions.vscode-nix-extensions.override {
        vscodeExtensionModule = config;
      };

    vscode-nix-extensions = final.callPackage ./pkgs/vscode-nix-extensions { };
  };
}
// builtins.listToAttrs (
  map (filename: {
    name =
      let
        m = builtins.match "(.+)\\.[^.]+" filename;
      in
      if m != null then builtins.head m else filename;
    value = lib'.lab.mkScript final scriptsDir filename;
  }) scriptFiles
)
