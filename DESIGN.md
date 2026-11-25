# Clipboard — Design System & Motion Specification

Clipboard is a zero-network, security-first, cross-platform desktop clipboard manager built with Native Shell (`nsh`).

## 1. Security Guarantee

- **Zero network calls**: No HTTP, socket, or cloud sync dependencies exist anywhere in the workspace. `nsh.json` declares `"capabilities": { "clipboard": true }` with no `network`, `webview`, or `openUrl` permissions.
- **Transient & concealed protection**: Clippings marked as transient or concealed by password managers (such as 1Password, Keychain Access, Passwords) are ignored automatically.
- **App blacklist**: Users can add or remove applications whose copies are never recorded.
- **Local-only persistence**: Settings and history are stored strictly in the OS local application-support directory with owner-only permissions (`0600` on Unix).

## 2. Supported Platforms & Targets

- **Primary target family: Desktop (macOS, Linux, Windows).**
  - Keyboard-first Quick Paste Overlay (`⇧⌘V` / `Ctrl+Shift+V`), compact macOS-inspired Preferences window (`520×500pt`) with 3-tab icon toolbar (`General`, `Shortcuts`, `Blacklist`), and system tray menu.
  - Full Light and Dark appearance support via `theme.json` semantic tokens (`theme()`).
  - Respects `reduce_motion()` everywhere; full keyboard navigation and accessibility roles/labels on every control.

## 3. Visual Language & Semantic Tokens

All colors in presentation code come from `theme()` (`theme.json`). No raw hex/rgb literals exist outside `theme.json`.

| Semantic Role | Purpose |
| --- | --- |
| `background` / `foreground` | Window canvas and primary text |
| `toolbar` / `toolbar-active` | Top preference toolbar surface and active tab pill |
| `popover` / `popover-foreground` | Floating Quick Paste Overlay and menus |
| `primary` / `primary-foreground` | Selected clipping row highlight (blue), active toolbar tab icon/label, checked checkboxes |
| `row-alt` / `section-header` | Alternating table rows and grouped shortcut section headers |
| `pill-surface` / `pill-accent` | Shortcut recorder pills (`+` / `⇧⌘V ×`) |
| `border` / `input` / `ring` | Hairline separators, input borders, keyboard focus rings |
| `muted` / `muted-foreground` | Helper text, search placeholder, italic shortcut index numbers |
| `success` / `warning` / `destructive` | Status badges and clear/remove confirmations |

## 4. App-Owned `ui/` Layer (`crates/clipboard-ui/src/ui/`)

Feature views compose exclusively from `crate::ui` and layout primitives (`view`, `text`, `scroll`, `image`).

- `tokens`: semantic colors, radii (`radius`, `radius-card`, `radius-inset`, `radius-overlay`), motion constants, monospace font.
- `button`: `action_button`, `icon_button`, `shortcut_pill_button`.
- `checkbox`: `checkbox_row` built on `nsh_base::checkbox`.
- `slider`: `history_slider` built on `nsh_base::slider` (100..=1000, step 100).
- `select`: `dropdown_select` built on `nsh_base::select`.
- `input`: `search_input`, `number_field` built on `nsh_base::input`.
- `surface`: `toolbar_tab`, `divider`, `badge`, `empty_state`.

## 5. Motion Tokens

| Token | Value | Use |
| --- | --- | --- |
| `DURATION_INSTANT` | `120.0` ms | Row hover wash, toolbar tab hover, focus ring |
| `DURATION_FAST` | `180.0` ms | Button press scale, checkbox fill, shortcut pill state |
| `DURATION_BASE` | `240.0` ms | Overlay selection highlight, preview drawer reveal |
| `PRESS_SCALE` | `0.97` | Standard buttons on active press |
| `ICON_PRESS_SCALE` | `0.95` | Square icon buttons on active press |

## 6. Motion Inventory (Per Screen)

### Quick Paste Overlay (`overlay.rs`)
| State Change | Tool & Tokens | Reduced Motion |
| --- | --- | --- |
| Overlay open / close | Floating `window()` panel toggle, Escape / blur dismiss | Instant |
| Row hover & selection highlight | `transition_colors()`, `DURATION_INSTANT` (120ms), `primary` fill on selected row | Instant swap |
| Selected row inline Filter / Preview buttons | `when` visibility on selection + `ICON_PRESS_SCALE` (`DURATION_FAST`) | No scale |
| Search input focus ring | `focus` border transition (`ring`, `DURATION_INSTANT`) | Instant |
| Preview / Filter panel toggle | `when` mount + `transition_colors()` (`DURATION_BASE`) | Instant swap |

### Preferences Shell & Pages (`preferences.rs`, `pages/{general,shortcuts,blacklist}.rs`)
| State Change | Tool & Tokens | Reduced Motion |
| --- | --- | --- |
| Toolbar tab switch (`General` / `Shortcuts` / `Blacklist`) | `transition_colors()`, `DURATION_INSTANT` (120ms), `toolbar-active` pill + `primary` icon/label | Instant swap |
| Checkboxes (`Always Paste Plain Text`, `Appear Behind Mouse Cursor`, `Release to Paste`, `Enable Interaction Sounds`, `Open automatically at login`) | `checkbox()` indicator `transition_colors()` (`DURATION_FAST`) | Instant swap |
| History limit slider (`100`–`1000`) | `slider()` thumb & range `transition_colors()` (`DURATION_FAST`) | Instant snap |
| Dropdown selects (`Quick Paste Menu`, `Dock and Menubar Icon`) | `select()` popup with `popover` surface & item hover wash | Instant |
| Shortcut pill record / clear | `pill-surface` hover wash (`DURATION_INSTANT`), active capture `ring` border | Instant |
| Blacklist row select & add/remove | Alternating `row-alt`, `primary` selection highlight, `+`/`−` press scale (`DURATION_FAST`) | No scale |
