# Slash Commands

ralf uses an **input-first** command model inspired by modern AI assistants like Claude Code. All typing goes directly to the input area, with no reserved keys that block text entry.

Commands are invoked by typing `/` followed by the command name. Type `/help` to see all available commands.

## Global Commands

These commands are always available:

| Command | Aliases | Description | Keybinding |
|---------|---------|-------------|------------|
| `/help` | `/?` | Show available commands | `F1` |
| `/quit` | `/q`, `/exit` | Exit ralf | `Esc` (when input empty) |
| `/split` | `/1` | Split view mode | `Ctrl+1` |
| `/focus` | `/2` | Focus conversation mode | `Ctrl+2` |
| `/canvas` | `/3` | Focus canvas mode | `Ctrl+3` |
| `/refresh` | | Refresh model status | `Ctrl+R` |
| `/clear` | | Clear conversation | `Ctrl+L` |
| `/search` | `/find` | Search timeline | `Ctrl+F` |
| `/model` | | Switch active model | |
| `/copy` | | Copy last response to clipboard | `Ctrl+C` |
| `/editor` | | Open in $EDITOR | |

## Phase-Specific Commands

These commands are only available during specific workflow phases:

### Review Phase

| Command | Aliases | Description |
|---------|---------|-------------|
| `/approve` | `/a` | Approve pending changes |
| `/reject` | `/r` | Reject with optional feedback |

### Running Phase

| Command | Description |
|---------|-------------|
| `/pause` | Pause running operation |
| `/cancel` | Cancel current operation |

### Paused Phase

| Command | Description |
|---------|-------------|
| `/resume` | Resume paused operation |
| `/cancel` | Cancel current operation |

### Drafting Phase

| Command | Description |
|---------|-------------|
| `/finalize` | Finalize the spec |
| `/assess` | Request AI assessment |

## Escape Cascade

The `Esc` key uses a cascading behavior:

1. **First press**: Clears the input if it contains text
2. **Second press**: Quits the application (when input is empty)

This allows you to quickly clear mistakes without accidentally quitting.

## Escaping Slashes

If you need to send a message that starts with `/` (like a file path), prefix it with an extra slash:

```
//etc/config    → sends "/etc/config" as a message
//help          → sends "/help" as a message (not the command)
```

## Input-First Model

ralf's command system follows these principles:

- **All character keys go to input**: No reserved keys block typing
- **Modifier keys for shortcuts**: `Ctrl+N` keybindings for power users
- **Slash commands for discoverability**: Type `/` to see what's available
- **Focus trap**: Pressing `/` from any pane jumps to the input

This design ensures you can always type freely without worrying about accidentally triggering actions.
