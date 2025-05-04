# zebu
![zebu](../../resources/mascots/zebu.jpg)


#### Development Guide

1. Install vivado

   Download Linux web installer
   https://www.xilinx.com/support/download/index.html/content/xilinx/en/downloadNav/vivado-design-tools.html

2. Enter dev shell

   ```sh
   nix develop .#zebu
   ```

3. Run the installer

4. Exit/Re-enter devShell

5. Run `vivado`

6. Follow this guide to create design and generate an xsa file

   https://www.hackster.io/whitney-knitter/getting-started-with-the-kria-kv260-in-vivado-2021-1-817ec2

7. Generate device trees

   ```sh
   nix develop .#zebu
   generate ./path/to/xsa
   ```

8. Add any extra dts config to ./hw/system-user.dts

9. Build the sdImage

    ```sh
    # You will need a remote builder for aarch64-linux or the following nixos configuration: 
    #
    #   boot.binfmt.emulatedSystems = [ "aarch64-linux" ];
    #
    nix build .#nixosConfigurations.zebu.sdImage
    ```
