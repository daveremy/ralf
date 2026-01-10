# Keybindings

ralf uses an **input-first** model where all character keys go to the input area. Global actions use modifier keybindings (`Ctrl+N`) to avoid blocking text entry.

## Global Keybindings

These keybindings work regardless of focus:

| Key | Action |
|-----|--------|
| `Esc` | Escape cascade (clear input → quit) |
| `F1` | Show help |
| `Ctrl+1` | Split view mode |
| `Ctrl+2` | Focus conversation mode |
| `Ctrl+3` | Focus canvas mode |
| `Ctrl+R` | Refresh model status |
| `Ctrl+L` | Clear conversation |
| `Ctrl+F` | Search timeline |
| `Ctrl+C` | Copy selected content |
| `Tab` | Switch focus between panes |

## Input Area

When typing in the input area:

| Key | Action |
|-----|--------|
| `Enter` | Submit input / execute command |
| `Shift+Enter` | Insert newline |
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character at cursor |
| `Left/Right` | Move cursor |
| `Home/End` | Move to start/end of line |
| `Up/Down` | Navigate input history |

## Timeline Navigation

When the timeline is focused:

| Key | Action |
|-----|--------|
| `Alt+j` | Select next event |
| `Alt+k` | Select previous event |
| `PageUp` | Page up |
| `PageDown` | Page down |

## Focus Trap

| Key | Action |
|-----|--------|
| `/` | Jump to input and start slash command |

Pressing `/` from any pane immediately focuses the input and inserts `/`, ready for you to type a command name.

## Philosophy

ralf's keybinding design follows these principles:

1. **Input-first**: Character keys always go to input
2. **Modifier shortcuts**: Power users get `Ctrl+N` shortcuts
3. **Discoverable**: Type `/` to see slash commands
4. **Safe defaults**: Hard to accidentally quit or lose work

For a complete list of commands, see [Slash Commands](./slash-commands.md).
