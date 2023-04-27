# home linux server

{ config, pkgs, lib, ... }:

with lib;
with builtins;

{
  imports = [ ../home.nix ];
}
