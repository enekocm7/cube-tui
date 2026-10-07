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
accent = "#FFD700"
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
big_timer = true
history = true
scramble = true
scramble_preview = false
stats = true
toasts = true
```

`history` controls the solve history panel, `stats` controls the statistics
panel, and `scramble` controls the scramble display. Hiding the history or
statistics panel reduces the minimum terminal width. Hiding the scramble
display reduces the minimum terminal height.

`big_timer` draws the timer in large block digits centered in the timer panel,
dropping leading zero minutes (`7.123` instead of `00:07.123`). In narrow panes
it switches to smaller digits, then to plain text. It defaults to `true`; set it
to `false` to show the time as a single line of text.

`scramble_preview` shows a colored ASCII net of the current scramble in the
bottom-right corner. Press `v` to toggle it; the choice is saved. Previews are
available for 2x2 through 7x7 cubes. Other events show "Preview is not available
for this puzzle". Cubes start with white on top and green in front, and sticker
letters identify colors. The preview refreshes when the scramble, event, or
session changes. Puzzle states and sticker geometry are computed directly in
Rust; the preview needs no generated move tables or external tooling.

The preview sits below statistics, or at the bottom right of the timer when
statistics are hidden. If there is not enough room for the complete net, it
shows a resize hint and keeps the stats visible. Larger puzzles widen the
right column and need more terminal rows. With the default layout, use a
terminal at least 27 rows tall for the 3x3 net.

`toasts` controls all toast notifications, including information, warnings,
and errors. It defaults to `true`; press `o` to toggle notifications and save
the choice, or set it to `false` to hide notifications in the configuration.
Turning notifications off clears any visible or queued toasts.

Completing a solve that sets a session best single, mo3, ao5, ao12, ao50, or
ao100 shows a toast. Records use the session's existing single and average statistics.
The first valid result establishes a record; ties do not count, and +2/DNF
penalties apply. Loading, importing, editing, and deleting do not trigger toasts.

Solves that set a **single** record use the theme's `accent` color in history,
even after a later solve beats them. A display flag on each solve tracks this;
it is saved with the solve and restored when loading history. Edits and deletions
recalculate the flags. Older solves without this field default to unmarked.
Average records use the accent only in the **current** stats
column while the latest average is a new session best. The **best** column
keeps its usual style. The accent defaults to gold for older themes that omit
it; selected record values stay bold and retain the selection background.

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
toggle_scramble_preview = "Ctrl+v"
toggle_toasts = "Ctrl+o"
help = "F1"
theme_selector = "Alt+t"
edit_solve = "F3"
```

Bindings accept a character or a named key: `Space`, `Enter`, `Esc`, `Tab`,
`BackTab`, `Up`, `Down`, `Left`, `Right`, `Backspace`, `Delete`, `Insert`,
`Home`, `End`, `PageUp`, `PageDown`, and `F1` through `F24`. Prefix a key
with `Ctrl+`, `Alt+`, or `Shift+`. Write shifted punctuation as the resulting
character, for example `?`. The timer binding must be an unmodified key.

Available actions are `quit`, `reset_timer`, `timer`, `select_up`,
`select_down`, `navigate_left`, `navigate_right`, `toggle_focus`, `next_event`,
`previous_event`, `next_session`, `previous_session`, `new_session`,
`delete_session`, `next_scramble`, `toggle_scramble_preview`, `help`, `toggle_inspection`,
`detailed_stats`, `theme_selector`, `delete_time`, `edit_solve`, `bluetooth`,
`disconnect_bluetooth`, `toggle_zen`, `toggle_toasts`, `enter`, and `back`.

Bindings are global and must be unique. Some actions are contextual: `timer`
also toggles a modifier in solve details, while `next_event` opens the selected
theme in the theme picker. Remove an override, or the entire `[keybinds]`
table, to restore its default. Some terminal-reserved key combinations may not
be delivered to the application.
