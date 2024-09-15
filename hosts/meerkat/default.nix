{ pkgs, ... }: {
  imports = [ ../../modules/darwin.nix ];

  environment.systemPackages = with pkgs; [ iterm2 ];

  home-manager = {
    useGlobalPkgs = true;
    useUserPackages = true;
    users.jmoore = _: {
      imports = [ ../../modules/home.nix ];

      lab = {
        direnv.enable = true;
        iterm2.enable = true;
        # karabiner.enable = true;
        nuphy75.enable = true;
        shell.enable = true;
        vscode.enable = true;
      };

      home = {
        homeDirectory = "/Users/jmoore";
        username = "jmoore";
        packages = with pkgs; [ spotify ];
        shellAliases = {
          switch = "darwin-rebuild switch --flake /Users/jmoore/Repos/lab";
        };
      };

      programs.yt-dlp.enable = true;
    };
  };

  programs.zsh.enable = true;

  users.users.jmoore = {
    name = "jmoore";
    home = "/Users/jmoore";
  };
}
