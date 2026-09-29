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
`delete_session`, `next_scramble`, `help`, `toggle_inspection`,
`detailed_stats`, `theme_selector`, `delete_time`, `edit_solve`, `bluetooth`,
`disconnect_bluetooth`, `toggle_zen`, `enter`, and `back`.

Bindings are global and must be unique. Some actions are contextual: `timer`
also toggles a modifier in solve details, while `next_event` opens the selected
theme in the theme picker. Remove an override, or the entire `[keybinds]`
table, to restore its default. Some terminal-reserved key combinations may not
be delivered to the application.

## Solve editor

Select a solve and press `Enter` to open its details, then `F3` to edit it.
The `edit_solve` binding is available **only in solve details** while the timer
is idle. The editor returns to those details when saved or cancelled.

- Use `Up`/`Down` or `Tab`/`Shift+Tab` to select a field, then `Enter` to edit.
- Use `Home`/`End` to jump to the first/last editor item or event-picker option.
- **Time** is the raw duration before a penalty. Enter seconds (`12.345`),
  minutes (`1:02.345`), or hours (`1:02:03.456`), with at most three decimal
  places. Durations must be positive and less than 24 hours; zero is allowed
  for a DNF.
- **Event** opens a picker of all supported puzzles.
- **Scramble** is required and validated against the selected event's move
  notation (up to 16384 bytes). This checks notation, not puzzle-state legality.
  When changing event, update the scramble to match before saving.
- **Penalty** cycles through None, +2 and DNF with `Enter` or `Left`/`Right`.
- **Solved at** accepts an RFC 3339 timestamp with a timezone, such as
  `2026-09-29T14:30:00.000Z`. Dates must be after the Unix epoch and use at most
  millisecond precision. Leave blank for an unknown date.
- **Comment** is optional and supports up to 4096 Unicode characters. The
  existing text input supports cursor editing and paste; pasted line breaks
  become spaces. Invalid submissions remain open with an error for correction.

Choose **Save changes** or press `Ctrl+S` to validate and save the entire draft.
`Esc` or **Cancel** discards it. While a field input or event picker is open,
`Esc` cancels just that input. The configured quit key (`q` by default) exits
from the editor, event picker, or change history, discarding unsaved edits.
While a text input is open, it captures shortcuts so letters such as `q` can
be entered normally.
Statistics and the last-time display are refreshed after a saved edit.

**Change history** displays timestamped before/after values, newest first. Use
the arrow keys, `PageUp`/`PageDown`, or `Home`/`End` to scroll. History survives
restarts and includes penalty toggles from solve details. Cancelled edits and
unchanged saves create no revisions. Older saved solves load with an empty
comment and history. Comments also round-trip through csTimer import/export;
the edit history stays in Cube TUI's native save file.
