# Settings

Cube TUI stores its settings in `config.toml`. Run `cube --config` to print
the path to the file. Restart Cube TUI after editing the configuration.

## Custom Theme

Run `cube --theme` to print the path to the themes directory. Theme files use
TOML and contain colors as six-digit hexadecimal values:

```toml
background = "#000000"
border = "#FFFFFF"
scramble = "#FFFFFF"
selection = "#3399FF"
selection_text = "#000000"
text = "#FFFFFF"
```

Create a `.toml` file in the themes directory using this format. Open the
theme selector with the configured `theme_selector` key, then choose the theme
to apply it. The default theme is created automatically as `default.toml`.

Or you can set the selected theme in `config.toml` with its file name:

```toml
[theme]
path = "my-theme.toml"
```

## Custom Layout

Use the `[display]` table in `config.toml` to choose which parts of the main
layout are visible:

```toml
[display]
history = true
scramble = true
stats = true
toasts = true
```

`history` controls the solve history panel, `stats` controls the statistics
panel, and `scramble` controls the scramble display. Hiding the history or
statistics panel reduces the minimum terminal width. Hiding the scramble
display reduces the minimum terminal height.

`toasts` controls all toast notifications, including information, warnings,
and errors. It defaults to `true`; set it to `false` to hide notifications.

## Timer Settings

Timer behavior is configured with the `[timer]` table:

```toml
[timer]
inspection = true
zen = false
```

`inspection` enables the WCA inspection period. `zen` hides the interface
while the timer is running.

## Custom Keybinds

Add a `[keybinds]` table containing only the bindings you want to change:

```toml
[keybinds]
next_scramble = "Ctrl+n"
help = "F1"
theme_selector = "Alt+t"
```

Bindings accept a character or a named key: `Space`, `Enter`, `Esc`, `Tab`,
`BackTab`, `Up`, `Down`, `Left`, `Right`, `Backspace`, `Delete`, `Insert`,
`Home`, `End`, `PageUp`, `PageDown`, and `F1` through `F24`. Prefix a key
with `Ctrl+`, `Alt+`, or `Shift+`. Write shifted punctuation as the resulting
character, for example `?`. The timer binding must be an unmodified key.

Available actions are `quit`, `reset_timer`, `timer`, `select_up`,
`select_down`, `navigate_left`, `navigate_right`, `toggle_focus`, `next_event`,
`previous_event`, `next_session`, `previous_session`, `new_session`,
`delete_session`, `next_scramble`, `help`, `toggle_inspection`,
`detailed_stats`, `theme_selector`, `delete_time`, `bluetooth`,
`disconnect_bluetooth`, `toggle_zen`, `enter`, and `back`.

Bindings are global and must be unique. Some actions are contextual: `timer`
also toggles a modifier in solve details, while `next_event` opens the selected
theme in the theme picker. Remove an override, or the entire `[keybinds]`
table, to restore its default. Some terminal-reserved key combinations may not
be delivered to the application.
