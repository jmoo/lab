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
    bitstream = ./hw/sdt/vivado_exported.bit;
    sdtDir = ./hw/sdt;
    dtDir = ./hw/dt;
  };

  hardware.deviceTree.overlays = [
    {
      name = "system-user";
      dtsFile = ./hw/system-user.dts;
    }
  ];
}
