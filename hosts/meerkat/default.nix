{ pkgs, inputs, ... }: {
  imports = [ ../../modules/darwin.nix ];

  environment.systemPackages = with pkgs; [ iterm2 ];
  
  home-manager = {
    useGlobalPkgs = true;
    useUserPackages = true;
    users.jmoore = _: {
      imports = [ ../../modules/home.nix ];
      
      lab = {
        iterm2.enable = true;
        shell.enable = true;
        vscode.enable = true;
      };
      
      home.homeDirectory = "/Users/jmoore";
      home.username = "jmoore";
    };
  };
            
  programs.zsh.enable = true;
  
  users.users.jmoore = {
    name = "jmoore";
    home = "/Users/jmoore";
  };
   
}
