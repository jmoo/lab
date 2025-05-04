{
  inputs,
  ...
}:
{
  imports = [
    inputs.nixos-xlnx.nixosModules.sd-image
  ];

  hardware.zynq = {
    platform = "zynqmp";
    bitstream = ./sdt/kria_kr260_bd_wrapper.bit;
    sdtDir = ./sdt;
    dtDir = ./dt;
  };

  hardware.deviceTree.overlays = [
    {
      name = "system-user";
      dtsFile = ./system-user.dts;
    }
  ];
}
