# Hyprlauncher Configuration Guide

Configuration file location: `~/.config/hyprlauncher/config.json`

## Configuration file

The configuration file controls the appearance and behavior of the launcher window.
```json
{
  "window": {
    "width": 600,                // Width of the launcher window in pixels
    "height": 600,               // Height of the launcher window in pixels
    "anchor": "center",          // Window position: "center", "top", "bottom", "left", "right", "top_left", "top_right", "bottom_left", "bottom_right"
    "margin_top": 0,             // Margin from the top of the screen in pixels
    "margin_bottom": 0,          // Margin from the bottom of the screen in pixels
    "margin_left": 0,            // Margin from the left of the screen in pixels
    "margin_right": 0,           // Margin from the right of the screen in pixels
    "show_descriptions": false,  // Show application descriptions in the list
    "show_paths": false,         // Show application paths in the list
    "show_icons": true,          // Show application icons in the list
    "show_search": true,         // Show the search bar
    "custom_navigate_keys": {    // Customize navigation key bindings
      "up": "k",                 // Key to move selection up
      "down": "j",               // Key to move selection down
      "delete_word": "h"         // Key to delete word in search
    },
    "show_border": true,         // Show window border
    "border_width": 2,           // Border width in pixels
    "use_gtk_colors": false,     // Use GTK theme colors instead of custom colors
    "max_entries": 50            // Maximum number of entries to show in the list
  },
  "theme": {
    "colors": {
      "border": "#333333",              // Border color in hex format
      "window_bg": "#0f0f0f",           // Window background color
      "search_bg": "#1f1f1f",           // Search bar background color
      "search_bg_focused": "#282828",   // Search bar background color when focused
      "item_bg": "#0f0f0f",             // List item background color
      "item_bg_hover": "#181818",       // List item background color on hover
      "item_bg_selected": "#1f1f1f",    // List item background color when selected
      "search_text": "#e0e0e0",         // Search text color
      "search_caret": "#808080",        // Search cursor color
      "item_name": "#ffffff",           // Application name color
      "item_description": "#a0a0a0",    // Application description color
      "item_path": "#808080"            // Application path color
    },
    "corners": {
      "window": 12,              // Window corner radius in pixels
      "search": 8,               // Search bar corner radius in pixels
      "list_item": 8             // List item corner radius in pixels
    },
    "spacing": {
      "search_margin": 12,       // Search bar outer margin in pixels
      "search_padding": 12,      // Search bar inner padding in pixels
      "item_margin": 6,          // List item outer margin in pixels
      "item_padding": 4          // List item inner padding in pixels
    },
    "typography": {
      "search_font_size": 16,               // Search bar font size in pixels
      "item_name_size": 14,                 // Application name font size in pixels
      "item_description_size": 12,          // Application description font size in pixels
      "item_path_size": 12,                 // Application path font size in pixels
      "item_path_font_family": "monospace"  // Font family for application paths
    }
  },
  "debug": {
    "disable_auto_focus": false,  // Disable automatic keyboard focus
    "enable_logging": false       // Enable application logging
  }
}
```
## Features

### Window Anchoring
The `anchor` setting determines where the window appears on screen. Options are:
- center: Window appears in the center of the screen
- top: Window appears at the top of the screen
- bottom: Window appears at the bottom of the screen
- left: Window appears on the left side of the screen
- right: Window appears on the right side of the screen
- top_left: Window appears in the top left corner
- top_right: Window appears in the top right corner
- bottom_left: Window appears in the bottom left corner
- bottom_right: Window appears in the bottom right corner

### Performance
- `max_entries`: Limits the maximum number of entries shown in the list for better performance

### Navigation Keys
Navigation can be customized using the `custom_navigate_keys` setting:
- `up`: Key to move selection up (default: "CTRL + k")
- `down`: Key to move selection down (default: "CTRL + j")
- `delete_word`: Key to delete word in search (default: "CTRL + h")

### Search
- The search bar can be focused by pressing `/`
- Escape clears the search or moves focus to the results list
- Supports fuzzy matching for application names
- Special path searching with `~`, `$`, or `/` prefixes
- Search results are ranked by launch frequency

### Visual Customization
- Border customization with `border_width` and `border_color`
- Corner radius customization for window, search bar, and list items
- Option to use GTK theme colors with `use_gtk_colors`
- Show/hide application icons, descriptions, and paths
- theme customization including colors, spacing, and typography

### Debug Options
- `disable_auto_focus`: Prevents the window from automatically holding all input
- `enable_logging`: Enables logging to the terminal window Hyprlauncher was launched from

## Hot Reloading
The configuration file is watched for changes and will automatically reload when modified. No need to restart the application.

> [!NOTE]
> To interact and see your live config changes while the launcher is open, set `disable_auto_focus` to `true` in your config:
> ```json
> {
>   "debug": {
>     "disable_auto_focus": true
>   }
> }
> ```
> This allows you to edit the config file while the launcher window is open. Otherwise, the launcher's exclusive keyboard focus will prevent text editing in other windows.

## Default Paths
Applications are searched in the following locations:
- ~/.local/share/applications
- /usr/share/applications
- /usr/local/share/applications
- /var/lib/flatpak/exports/share/applications
- ~/.local/share/flatpak/exports/share/applications

Furthermore, applications can be indexed via XDG_DATA_DIRS environment variable.

## Application Launch History
Hyprlauncher maintains a launch history for applications in `~/.local/share/hyprlauncher/heatmap.json`. This is used to improve search result rankings based on usage frequency.

## Config Merging
If the configuration file is invalid or missing certain values, Hyprlauncher will:
1. Use default values for missing fields
2. Merge existing valid configuration with defaults
3. Write the merged configuration back to the file

The configuration file is strict and requires valid JSON format. Invalid configurations will fall back to defaults.

