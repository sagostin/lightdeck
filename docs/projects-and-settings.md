# Projects and settings

## Projects

A project holds everything about a show: fixture types, patch, rooms, groups, outputs, audio bands, routes, palettes, presets, controls, MIDI and Zigbee settings, and your Zigbee device names.

The project name at the top of the sidebar is the **File** menu:

| Item | Shortcut |
|---|---|
| New project | |
| Open… | `⌘O` |
| Save | `⌘S` |
| Save as… | `⇧⌘S` |
| Recent | The last 8 projects |

Projects are saved as `.lightdeck` files (JSON). A dot next to the project name, and in the window title, means there are unsaved changes. If you have unsaved changes, New and Open ask you to click again before discarding them.

### Autosave

Every change is also saved straight away to a **working copy**, so nothing is lost if the app closes. Saving writes the working copy to the project file. On the next launch LightDeck reopens the working copy.

If the working copy can't be read (for example after a crash during a write), LightDeck keeps it aside as `show.unreadable-<time>.json` instead of overwriting it, and starts the starter show.

Older projects are upgraded automatically when opened: new detection bands, multi-range bands, palettes and so on are added without changing what you'd set.

## App settings

Some settings belong to LightDeck on this Mac rather than to a project:

- the AI provider (base URL, schema, model, headers, key);
- your saved band presets;
- the recent-projects list.

**Export settings…** in the File menu writes a `.lightdeck-settings` file with the AI settings and band presets. The API key is left out unless you tick **Include API key in export**. **Import settings…** merges band presets by name and replaces the AI settings, keeping your current key if the file has none.

## Where data lives

Everything is in `~/Library/Application Support/dev.lightdeck.app/`:

| Path | Contents |
|---|---|
| `show.json` | The autosaved working copy of the current show |
| `prefs.json` | Current project path, recent projects, band presets |
| `ai.json` | AI provider settings, **including the API key** in plain text |
| `zigbee/network.json` | The USB Zigbee network's PAN ID, extended PAN ID, network key and channel |
| `zigbee/database.db`, `coordinator_backup.json` | Paired Zigbee devices and the coordinator backup |

Back up the `zigbee` folder; losing it means re-pairing every device. Projects can live anywhere you like.
