# Karabiner config for my nuphy75 keyboard
{ config, lib, ... }:

with lib;
with builtins;
with config.lib.karabiner;

{
  options.lab.nuphy75.enable = mkEnableOption "nuphy75";

  # Key-mapping in karabiner for my nuphy air75 keyboard
  config.lab.karabiner.profiles.default = mkIf config.lab.nuphy75.enable {
    complex_modifications = {
      rules = [
        {
          description =
            "Map fn-spacebar to ⌥-spacebar because iterm2 can't detect fn keys";

          manipulators = [{
            type = "basic";

            from.key_code = "spacebar";
            from.modifiers.mandatory = "fn";

            to = [{
              key_code = "spacebar";
              modifiers = "left_option";
            }];
          }];
        }

        {
          description = "Map option-down to ctrl-right for workspace switching";

          manipulators = [{
            type = "basic";

            from.key_code = "down_arrow";
            from.modifiers.mandatory = "left_option";

            to = [{
              key_code = "right_arrow";
              modifiers = "left_control";
            }];
          }];
        }

        {
          description = "Map option-up to ctrl-left for workspace switching";

          manipulators = [{
            type = "basic";

            from.key_code = "up_arrow";
            from.modifiers.mandatory = "left_option";

            to = [{
              key_code = "left_arrow";
              modifiers = "left_control";
            }];
          }];
        }
      ];
    };

    devices = [{
      identifiers = {
        product_id = 591;
        vendor_id = 1452;
      };

      simple_modifications = [
        (mapKey "left_command" "left_control")
        (mapKey "left_control" "left_command")
      ];
    }];
  };
}
