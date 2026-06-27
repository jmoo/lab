inputs: final: prev:
let
  scriptsDir = ./scripts;
  scriptsEntries = builtins.readDir scriptsDir;
  scriptFiles = builtins.filter (
    name: scriptsEntries.${name} == "regular" && prev.lib.hasSuffix ".sh" name
  ) (builtins.attrNames scriptsEntries);

  mkScript =
    filename:
    let
      name = prev.lib.removeSuffix ".sh" filename;
      src = builtins.readFile (scriptsDir + "/${filename}");
      lines = prev.lib.splitString "\n" src;

      depsLine = prev.lib.findFirst (l: prev.lib.hasPrefix "# deps:" l) null lines;
      runtimeInputs =
        if depsLine != null then
          map (d: final.${d}) (
            builtins.filter (s: s != "") (prev.lib.splitString " " (prev.lib.removePrefix "# deps: " depsLine))
          )
        else
          [ ];

      # Strip shebang so writeShellApplication can supply its own.
      body = prev.lib.concatStringsSep "\n" (
        if lines != [ ] && prev.lib.hasPrefix "#!" (builtins.head lines) then builtins.tail lines else lines
      );
    in
    final.writeShellApplication {
      inherit name runtimeInputs;
      text = body;
    };
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
    name = prev.lib.removeSuffix ".sh" filename;
    value = mkScript filename;
  }) scriptFiles
)
