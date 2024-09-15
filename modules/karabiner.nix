{
  config,
  pkgs,
  lib,
  ...
}:

with lib;
with builtins;

let
  # Sub-module type for freeform json documents
  json = (pkgs.formats.json { }).type;

  # Simple function for easy key mapping
  mapKey = from: to: {
    from =
      if isString from then
        {
          key_code = from;
        }
      else if isAttrs from then
        from
      else
        throw "Invalid type for mapKey -> from";

    to =
      if isList to then
        (map (
          x:
          if isAttrs x then
            x
          else if isString to then
            {
              key_code = x;
            }
          else
            throw "Invalid type for mapKey -> to [x]"
        ) to)
      else if isAttrs to then
        [ to ]
      else if isString to then
        [
          {
            key_code = to;
          }
        ]
      else
        throw "Invalid type for mapKey -> to";
  };

  mapDefaults =
    defaults:
    mapAttrs (
      name: value:
      mkOption {
        type = types.anything;
        default = value;
      }
    ) defaults;

  # Default karabiner settings
  defaults = {
    global = {
      ask_for_confirmation_before_quitting = true;
      check_for_updates_on_startup = true;
      show_in_menu_bar = true;
      show_profile_name_in_menu_bar = false;
      unsafe_ui = false;
    };

    profile = {
      name = "home-manager profile";

      complex_modifications.parameters = {
        "basic.simultaneous_threshold_milliseconds" = 50;
        "basic.to_delayed_action_delay_milliseconds" = 500;
        "basic.to_if_alone_timeout_milliseconds" = 1000;
        "basic.to_if_held_down_threshold_milliseconds" = 500;
        "mouse_motion_to_scroll.speed" = 100;
      };

      parameters = {
        delay_milliseconds_before_open_device = 1000;
      };

      virtual_hid_keyboard = {
        country_code = 0;
        indicate_sticky_modifier_keys_state = true;
        mouse_key_xy_scale = 100;
      };

      device = {
        disable_built_in_keyboard_if_exists = false;
        ignore = false;
        manipulate_caps_lock_led = true;
        treat_as_built_in_keyboard = false;

        identifiers = {
          is_keyboard = true;
          is_pointing_device = false;
        };
      };
    };
  };

  # Schema for a karabiner device
  kdevice = types.submodule {
    freeformType = json;

    options = (mapDefaults (removeAttrs defaults.profile.device [ "identifiers" ])) // {
      identifiers = mkOption {
        type = types.submodule {
          freeformType = json;

          options = (mapDefaults defaults.profile.device.identifiers) // {
            product_id = mkOption { type = types.anything; };

            vendor_id = mkOption { type = types.anything; };
          };
        };
      };
    };
  };

  # Schema for a karabiner profile
  kprofile = types.submodule {
    freeformType = json;

    options = {
      selected = mkEnableOption "selected";

      name = mkOption { type = types.str; };

      complex_modifications = mkOption {
        type = types.submodule {
          freeformType = json;

          options = {
            parameters = mkOption {
              type = types.submodule {
                freeformType = json;
                options = mapDefaults defaults.profile.complex_modifications.parameters;
              };

              default = { };
            };

            rules = mkOption {
              type = types.listOf (types.submodule { freeformType = json; });

              default = [ ];
            };
          };
        };
      };

      parameters = mkOption {
        type = types.submodule {
          freeformType = json;
          options = mapDefaults defaults.profile.parameters;
        };

        default = { };
      };

      devices = mkOption {
        type = types.listOf kdevice;
        default = [ ];
      };

      fn_function_keys = mkOption {
        type = types.listOf (types.submodule { freeformType = json; });
        default = [ ];
      };

      simple_modifications = mkOption {
        type = types.listOf (types.submodule { freeformType = json; });
        default = [ ];
      };

      virtual_hid_keyboard = mkOption {
        type = types.submodule {
          freeformType = json;
          options = mapDefaults defaults.profile.virtual_hid_keyboard;
        };

        default = { };
      };
    };
  };

  # Schema for entire karabiner settings file
  ksettings = types.submodule {
    freeformType = json;

    options = {
      global = mkOption {
        type = types.submodule {
          freeformType = json;
          options = mapDefaults defaults.global;
        };

        default = { };
      };

      profiles = mkOption {
        type = types.listOf kprofile;
        default = [ ];
      };
    };
  };

in
{
  config.lib.karabiner = {
    inherit mapKey;
  };

  options.lab.karabiner = {
    enable = mkOption {
      description = "Enable karabiner.json";
      type = types.bool;
      default = false;
    };

    settings = mkOption {
      description = "Compiled karabiner.json contents";
      type = ksettings;
      default = { };
    };

    profiles = mkOption {
      type = types.attrsOf kprofile;
      default = { };
    };

    file = mkOption {
      description = "Generated karabiner.json";
      type = types.package;
    };
  };

  config.lab.karabiner.profiles.default.name = mkDefault "Home manager profile";
  config.lab.karabiner.profiles.default.selected = mkDefault true;

  # Default karabiner settings
  config.lab.karabiner.settings = {
    profiles = attrValues config.lab.karabiner.profiles;
  };

  # Generated karabiner.json file from settings
  config.lab.karabiner.file = pkgs.writeTextFile {
    name = "karabiner.json";
    text = toJSON config.lab.karabiner.settings;
  };

  # Copy file to karabiner config directory
  config.home.file.karabiner-json = mkIf config.lab.karabiner.enable {
    executable = false;
    source = config.lab.karabiner.file;
    target = ".config/karabiner/karabiner.json";
  };
}
