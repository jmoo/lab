{
  inputs,
  runtimeShell,
  callPackage,
  writeShellScriptBin,
  nixos-xlnx,
  xorg,
  ...
}:
let
  # Generate device trees
  generate = writeShellScriptBin "generate" ''
    #!${runtimeShell}
    xsaFile="$1"
    repoRoot="$(git rev-parse --show-toplevel)"

    ${nixos-xlnx}/bin/gendt.tcl "''${xsaFile:-"$HOME/vivado/zebu/kria_kr260_bd_wrapper.xsa"}" "$repoRoot/hosts/zebu/hw" -platform zynqmp 
  '';
in
callPackage "${inputs.nix-environments}/envs/xilinx-vitis/shell.nix" {
  extraPkgs = [
    generate
    xorg.xlsclients
  ];
}
