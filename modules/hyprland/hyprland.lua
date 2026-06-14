-- Monitors
-- Samsung C49RG9x ultrawide — full resolution and refresh rate (matched by panel, not port)
hl.monitor({ output = "desc:Samsung Electric Company C49RG9x H4ZN500416", mode = "5120x1440@119.97", position = "0x0", scale = 1 })
hl.monitor({ output = "desc:Samsung Electric Company C49RG9x H1AK500000", mode = "5120x1440@119.97", position = "0x0", scale = 1 })
-- Fallback for any other monitor
hl.monitor({ output = "", mode = "preferred", position = "auto", scale = "auto" })

-- Environment variables
hl.env("XCURSOR_SIZE", "24")
hl.env("HYPRCURSOR_SIZE", "24")

-- Start terminal in special workspace
hl.on("hyprland.start", function()
  hl.exec_cmd("[workspace special:magic silent] " .. terminal)
end)

-- Look and feel
hl.config({
  general = {
    gaps_in = 5,
    gaps_out = 10,
    border_size = 2,
    col = {
      active_border   = { colors = { "rgba(33ccffee)", "rgba(00ff99ee)" }, angle = 45 },
      inactive_border = "rgba(595959aa)",
    },
    resize_on_border = true,
    allow_tearing    = false,
    layout           = "dwindle",
  },

  decoration = {
    rounding         = 10,
    active_opacity   = 1.0,
    inactive_opacity = 1.0,
    shadow = {
      enabled      = true,
      range        = 4,
      render_power = 3,
      color        = "rgba(1a1a1aee)",
    },
    blur = {
      enabled  = true,
      size     = 3,
      passes   = 1,
      vibrancy = 0.1696,
    },
  },

  animations = {
    enabled = true,
  },

  dwindle = {
    preserve_split = true,
  },

  master = {
    new_status = "master",
  },
})

-- Bezier curves
hl.curve("easeOutQuint",   { type = "bezier", points = { { 0.23, 1    }, { 0.32, 1 } } })
hl.curve("easeInOutCubic", { type = "bezier", points = { { 0.65, 0.05 }, { 0.36, 1 } } })
hl.curve("linear",         { type = "bezier", points = { { 0,    0    }, { 1,    1 } } })
hl.curve("almostLinear",   { type = "bezier", points = { { 0.5,  0.5  }, { 0.75, 1 } } })
hl.curve("quick",          { type = "bezier", points = { { 0.15, 0    }, { 0.1,  1 } } })

-- Animations
hl.animation({ leaf = "global",        enabled = true, speed = 10,   bezier = "default"       })
hl.animation({ leaf = "border",        enabled = true, speed = 5.39, bezier = "easeOutQuint"  })
hl.animation({ leaf = "windows",       enabled = true, speed = 4.79, bezier = "easeOutQuint"  })
hl.animation({ leaf = "windowsIn",     enabled = true, speed = 4.1,  bezier = "easeOutQuint",  style = "popin 87%" })
hl.animation({ leaf = "windowsOut",    enabled = true, speed = 1.49, bezier = "linear",        style = "popin 87%" })
hl.animation({ leaf = "fadeIn",        enabled = true, speed = 1.73, bezier = "almostLinear"  })
hl.animation({ leaf = "fadeOut",       enabled = true, speed = 1.46, bezier = "almostLinear"  })
hl.animation({ leaf = "fade",          enabled = true, speed = 3.03, bezier = "quick"         })
hl.animation({ leaf = "layers",        enabled = true, speed = 3.81, bezier = "easeOutQuint"  })
hl.animation({ leaf = "layersIn",      enabled = true, speed = 4,    bezier = "easeOutQuint",  style = "fade" })
hl.animation({ leaf = "layersOut",     enabled = true, speed = 1.5,  bezier = "linear",        style = "fade" })
hl.animation({ leaf = "fadeLayersIn",  enabled = true, speed = 1.79, bezier = "almostLinear"  })
hl.animation({ leaf = "fadeLayersOut", enabled = true, speed = 1.39, bezier = "almostLinear"  })
hl.animation({ leaf = "workspaces",    enabled = true, speed = 1.94, bezier = "almostLinear",  style = "fade" })
hl.animation({ leaf = "workspacesIn",  enabled = true, speed = 1.21, bezier = "almostLinear",  style = "fade" })
hl.animation({ leaf = "workspacesOut", enabled = true, speed = 1.94, bezier = "almostLinear",  style = "fade" })

-- Input
hl.config({
  input = {
    kb_layout  = "us",
    kb_variant = "",
    kb_model   = "",
    kb_options = "",
    kb_rules   = "",
    follow_mouse = 1,
    sensitivity  = 0,
    touchpad = {
      drag_lock      = 0,
      natural_scroll = true,
    },
  },
})

hl.gesture({ fingers = 3, direction = "horizontal", action = "workspace" })

hl.device({ name = "epic-mouse-v1", sensitivity = -0.5 })

-- Keybindings
hl.bind(mod .. " + T",      hl.dsp.exec_cmd(terminal))
hl.bind(mod .. " + Q",      hl.dsp.window.close())
hl.bind(mod .. " + R",      hl.dsp.exec_cmd("hyprctl reload"))
hl.bind(modCtrl .. " + delete",    hl.dsp.exec_cmd("uwsm stop"))
hl.bind(modCtrl .. " + backspace", hl.dsp.exec_cmd("uwsm stop"))
hl.bind(mod .. " + F",      hl.dsp.window.fullscreen())
hl.bind(mod .. " + D",      hl.dsp.exec_cmd(launcher))
hl.bind(mod .. " + P",      hl.dsp.window.pseudo())
hl.bind(mod .. " + S",      hl.dsp.layout("togglesplit"))
hl.bind(mod .. " + L",      hl.dsp.exec_cmd(lock))
hl.bind(mod .. " + C",      hl.dsp.window.float({ action = "toggle" }))
hl.bind(mod .. " + C",      hl.dsp.window.center())
-- Fallback: always opens a terminal regardless of GPU or config state
hl.bind(mod .. " + X",      hl.dsp.exec_cmd("xterm"))

-- Special workspace (scratchpad)
hl.bind(mod      .. " + space", hl.dsp.workspace.toggle_special("magic"))
hl.bind(modShift .. " + space", hl.dsp.window.move({ workspace = "special:magic" }))

-- Window resize with arrow keys
hl.bind(modAlt .. " + left",  hl.dsp.window.resize({ x = 10, y = 0,  relative = true }))
hl.bind(modAlt .. " + right", hl.dsp.window.resize({ x = 10, y = 0,  relative = true }))

-- Move focus
hl.bind(mod .. " + left",  hl.dsp.focus({ direction = "l" }))
hl.bind(mod .. " + right", hl.dsp.focus({ direction = "r" }))
hl.bind(mod .. " + up",    hl.dsp.focus({ direction = "u" }))
hl.bind(mod .. " + down",  hl.dsp.focus({ direction = "d" }))

-- Swap windows
hl.bind(modShift .. " + left",  hl.dsp.window.swap({ direction = "l" }))
hl.bind(modShift .. " + right", hl.dsp.window.swap({ direction = "r" }))
hl.bind(modShift .. " + up",    hl.dsp.window.swap({ direction = "u" }))
hl.bind(modShift .. " + down",  hl.dsp.window.swap({ direction = "d" }))

-- Switch / move to workspaces 1–10
for i = 1, 10 do
  local key = i % 10  -- 10 maps to key 0
  hl.bind(mod      .. " + " .. key, hl.dsp.focus({ workspace = i }))
  hl.bind(modShift .. " + " .. key, hl.dsp.window.move({ workspace = i }))
end

-- Scroll through workspaces
hl.bind(mod .. " + mouse_down", hl.dsp.focus({ workspace = "e+1" }))
hl.bind(mod .. " + mouse_up",   hl.dsp.focus({ workspace = "e-1" }))

hl.bind(modShiftCtrl .. " + left",  hl.dsp.window.move({ workspace = "e-1" }))
hl.bind(modShiftCtrl .. " + right", hl.dsp.window.move({ workspace = "e+1" }))

hl.bind(modCtrl .. " + left",  hl.dsp.focus({ workspace = "e-1" }))
hl.bind(modCtrl .. " + right", hl.dsp.focus({ workspace = "e+1" }))

-- Move/resize with mouse
hl.bind(mod .. " + mouse:272", hl.dsp.window.drag(),   { mouse = true })
hl.bind(mod .. " + mouse:273", hl.dsp.window.resize(), { mouse = true })

-- Multimedia keys
hl.bind("XF86AudioRaiseVolume",  hl.dsp.exec_cmd("wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+"), { locked = true, repeating = true })
hl.bind("XF86AudioLowerVolume",  hl.dsp.exec_cmd("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"),      { locked = true, repeating = true })
hl.bind("XF86AudioMute",         hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"),     { locked = true, repeating = true })
hl.bind("XF86AudioMicMute",      hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"),   { locked = true, repeating = true })
hl.bind("XF86MonBrightnessUp",   hl.dsp.exec_cmd("brightnessctl s 10%+"),                           { locked = true, repeating = true })
hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd("brightnessctl s 10%-"),                           { locked = true, repeating = true })

hl.bind("XF86AudioNext",  hl.dsp.exec_cmd("playerctl next"),        { locked = true })
hl.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"),  { locked = true })
hl.bind("XF86AudioPlay",  hl.dsp.exec_cmd("playerctl play-pause"),  { locked = true })
hl.bind("XF86AudioPrev",  hl.dsp.exec_cmd("playerctl previous"),    { locked = true })

-- Window rules
hl.window_rule({ name = "pavucontrol",  float = true, pin = true, size = "500 700", move = "100%-525 80",  match = { class = "org.pulseaudio.pavucontrol" } })
hl.window_rule({ name = "blueman",      float = true, pin = true, size = "500 700", move = "100%-525 80",  match = { class = ".blueman-manager-wrapped" } })
hl.window_rule({ name = "nm-editor",   float = true, pin = true, size = "500 700", move = "100%-525 80",  match = { class = "nm-connection-editor" } })
hl.window_rule({ name = "nm-applet",   float = true, pin = true, size = "500 700", move = "100%-525 80",  match = { class = "nm-applet" } })
hl.window_rule({ name = "ulauncher-prefs", float = true, pin = true, size = "600 800", move = "100%-900 80", match = { class = "ulauncher", title = "Ulauncher Preferences" } })

-- Ignore maximize requests from apps
hl.window_rule({ name = "suppress-maximize", suppress_event = "maximize", match = { class = ".*" } })

-- Fix dragging issues with XWayland
hl.window_rule({
  name  = "xwayland-nofocus",
  no_focus = true,
  match = {
    class      = "^$",
    title      = "^$",
    xwayland   = true,
    float      = true,
    fullscreen = false,
    pin        = false,
  },
})
