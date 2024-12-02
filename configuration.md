# Hyprlauncher Configuration Guide

> [!WARNING]
> This documentation represents the latest development version from Git, not the latest release. Some features described here may not be available in the current release version.

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
    "show_actions": false,       // Show additional application actions in the list
    "custom_navigate_keys": {    // Customize navigation key bindings
      "up": "k",                 // Key to move selection up
      "down": "j",               // Key to move selection down
      "delete_word": "h"         // Key to delete word in search
    },
    "show_border": true,         // Show window border
    "border_width": 2,           // Border width in pixels
    "use_gtk_colors": false,     // Use GTK theme colors instead of custom colors
    "use_custom_css": false,     // Use custom CSS file for styling
    "max_entries": 50            // Maximum number of entries to show in the list
  },
  "theme": {
    "colors": {
      "border": "#333333",                    // Border color in hex format
      "window_bg": "#0f0f0f",                 // Window background color
      "search_bg": "#1f1f1f",                 // Search bar background color
      "search_bg_focused": "#282828",         // Search bar background color when focused
      "item_bg": "#0f0f0f",                   // List item background color
      "item_bg_hover": "#181818",             // List item background color on hover
      "item_bg_selected": "#1f1f1f",          // List item background color when selected
      "search_text": "#e0e0e0",               // Search text color
      "search_caret": "#808080",              // Search cursor color
      "item_name": "#ffffff",                 // Application name color
      "item_name_selected": "#ffffff",        // Application name color when selected
      "item_description": "#a0a0a0",          // Application description color
      "item_description_selected": "#a0a0a0", // Application description color when selected
      "item_path": "#808080",                 // Application path color
      "item_path_selected": "#808080",        // Application path color when selected
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
  },
  "dmenu": {
    "allow_invalid": false,      // Allow invalid entries when no matches are found
    "case_sensitive": false      // Enable case-sensitive matching
  },
  "web_search": {
    "enabled": false,          // Enable/disable web search functionality
    "engine": "duckduckgo",    // Use preset engine name
    "prefixes": [              // Custom search prefixes
      {
        "prefix": "yt",
        "url": "https://www.youtube.com/results?search_query="
      },
      {
        "prefix": "gh",
        "url": "https://github.com/search?q="
      }
    ]
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

### Application Actions
Desktop entries can define additional actions that appear as separate entries when `show_actions` is enabled in the config. Actions allow quick access to specific application features, for example:
- Firefox's private browsing mode
- Terminal's new window/tab options
- Custom application-specific commands

To enable actions, set `show_actions` to `true` in your config:
```json
{
  "window": {
    "show_actions": true
  }
}
```

Actions will appear as separate entries with the format "Application Name - Action Name".

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
- Border customization with `border_width` - Window section, and `border` - Theme section
- Corner radius customization for window, search bar, and list items
- Option to use GTK theme colors with `use_gtk_colors`
- Show/hide application icons, descriptions, and paths
- theme customization including colors, spacing, and typography

### Debug Options
- `disable_auto_focus`: Prevents the window from automatically holding all input
- `enable_logging`: Enables logging to the terminal window Hyprlauncher was launched from

### Dmenu Mode
- Alternative mode that mimics dmenu functionality
- Launch with `--dmenu` or `-d` flag
- Reads input lines from stdin and presents them in a searchable list
- Configuration options:
  - `allow_invalid`: When enabled, allows entering text that doesn't match any entry
  - `case_sensitive`: When enabled, performs case-sensitive matching

### Web Search
- Enables web search functionality when no matching applications are found
- Configuration options in the `web_search` section:
  ```json
  {
    "web_search": {
      "enabled": false,          // Enable/disable web search functionality
      "engine": "duckduckgo",    // Use preset engine name
      "prefixes": [              // Custom search prefixes
        {
          "prefix": "yt",
          "url": "https://www.youtube.com/results?search_query="
        },
        {
          "prefix": "gh",
          "url": "https://github.com/search?q="
        }
      ]
    }
  }
  ```
- Available preset engines:
  - `duckduckgo` (default)
  - `google`
  - `bing`
  - `brave`
  - `ecosia`
  - `startpage`
- Custom search engines:
  - Provide the full search URL with the query parameter placeholder
  - The URL must end with the query parameter (e.g., `?q=`, `?query=`, `?search=`)
  - The search term will be automatically URL-encoded and appended to this URL
- Search prefixes:
  - Use `prefix:query` format to search with a specific engine
  - Example: `yt:how to code` searches YouTube
  - Example: `gh:rust-lang` searches GitHub
  - Prefixes are defined in the config file

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

## Terminal Applications
Terminal-based application launching requires the `TERMINAL` environment variable to be set. If not set, Hyprlauncher will fall back to using `xterm`. To ensure terminal applications launch properly, set your terminal emulator:

```bash
export TERMINAL=alacritty  # or kitty, foot, etc.
```

This can be added to your shell's profile (e.g., `.bash_profile`, `.zprofile`).

## Application Launch History
Hyprlauncher maintains a launch history for applications in `~/.local/share/hyprlauncher/heatmap.json`. This is used to improve search result rankings based on usage frequency.

## Config Merging
If the configuration file is invalid or missing certain values, Hyprlauncher will:
1. Use default values for missing fields
2. Merge existing valid configuration with defaults
3. Write the merged configuration back to the file

The configuration file is strict and requires valid JSON format. Invalid configurations will fall back to defaults.

## Custom Styling
Hyprlauncher supports two methods of styling:

1. Built-in Theme Configuration (Default)
- Uses the theme settings from config.json
- Provides a simple way to customize colors, spacing, and typography
- Recommended for basic customization needs

2. Custom CSS
- Enables full control over the application's appearance
- Activated by setting `use_custom_css: true` in config.json
- CSS file location: `~/.config/hyprlauncher/style.css`

To use custom CSS:

1. Enable custom CSS in your config.json:
```json
{
  "window": {
    "use_custom_css": true
  }
}
```

2. Create a style.css file in your Hyprlauncher config directory:

```bash
touch ~/.config/hyprlauncher/style.css
```

3. Add your custom CSS rules. Example style.css:
```css
window {
    background-color: #1a1b26;
    border-radius: 12px;
    border: 2px solid #414868;
}

listview {
    background: transparent;
}

listview > row {
    padding: 8px;
    margin: 4px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.05);
    transition: all 200ms ease;
}

listview > row:selected {
    background-color: #7aa2f7;
}

entry {
    margin: 12px;
    padding: 8px 12px;
    border-radius: 6px;
    background-color: rgba(255, 255, 255, 0.1);
    color: #c0caf5;
    caret-color: #7aa2f7;
    font-size: 16px;
}

.app-name {
    color: #c0caf5;
    font-size: 14px;
    font-weight: bold;
}

.app-description {
    color: #565f89;
    font-size: 12px;
}

.app-path {
    color: #414868;
    font-size: 12px;
    font-family: monospace;
}
```

Available CSS Classes:
- `window`: Main application window
- `listview`: Application list container
- `entry`: Search input field
- `.app-name`: Application name text
- `.app-description`: Application description text
- `.app-path`: Application path text

GTK CSS Properties:
All standard GTK4 CSS properties are supported. Common properties include:
- `background-color`
- `color`
- `border`
- `border-radius`
- `margin`
- `padding`
- `font-family`
- `font-size`
- `font-weight`
- `transition`
- `opacity`
- `box-shadow`

When `use_custom_css` is enabled, all theme settings from config.json are ignored in favor of your custom CSS rules.
