# TinyWorld

Desktop pets for Windows: a **Tauri v2 + Vite + TypeScript** app with a Rust backend. The pets are drawn
on a transparent overlay window that always stays on top and lets the mouse pass through except where it
points at a pet. Roadmap and settled decisions: [PLAN.md](PLAN.md).

The other docs in this repo (PLAN.md, CREATE_PET.md, AGENTS.md, the READMEs under `assets/` and
`website/`) and the code comments are in Vietnamese.

```
desktop-pet/
├── assets/
│   ├── icon.png          # the app's source logo
│   ├── sprites/          # sprite packs the app runs: <pack>/pet.json + WebP
│   └── sprite-sources/   # source atlases and prompts for each character, plus an animation viewer
├── packages/
│   ├── core/             # types shared by Rust and TS, event names, reading/validating pet.json
│   └── sim/              # pure TS simulation engine: FSM, fixed time step, seeded RNG
├── desktop/              # Tauri v2
│   ├── src/              # overlay (draws pets on a canvas, plain TS) and src/settings/ (settings window, React)
│   └── src-tauri/        # Rust: overlay, click-through, cursor tracking, tray, settings, saved state
└── scripts/              # normalize sprites from source atlases, generate icons
```

## Characters

22 original characters drawn for the project. Pick them in **Settings… → Pets** and click a tile to add or
remove that character: up to 3 can live on the screen at once, and at least one always stays (16
characters per page; flip pages with the arrows, the page dots or the mouse wheel). They are Momo (the
default), Bông, Kitsu, Mầm, Bíp, Lumi, Nấm, Mây, Tàn, Rêu, Cục, Mực, Dứa, Su, Bắp, Boggo, Gloop, Bẹp,
Frobu, Long, Byte and Patch. Every character has all 12 animations: the 9 from Phase 1 plus climb, perch
and jump. Watch them move in the [sprite studio](assets/sprite-sources/index.html) (open it straight in a
browser); each character's design is in [assets/sprite-sources/README.md](assets/sprite-sources/README.md).

The `pet.json` format: [assets/README.md](assets/README.md). The full guide to creating and adding a new
character: [CREATE_PET.md](CREATE_PET.md).

## Requirements

- Node.js 22+, pnpm 11+ (pnpm 10 switches itself to the version in `packageManager`)
- Rust (`x86_64-pc-windows-msvc` toolchain) and Visual Studio Build Tools (C++). The
  `x86_64-pc-windows-gnu` toolchain also builds, but the linker warns about merging manifests; CI uses MSVC.
- WebView2 Runtime (preinstalled on recent Windows 10/11)

## Running in dev

```bash
pnpm install
pnpm dev:desktop   # the first Rust compile takes a few minutes
```

The app has no window on the taskbar. The first time, the pet appears in the bottom-right corner, standing
on the edge of the taskbar; after that it comes back exactly where it was when the app closed (still
asleep if it was sleeping). Control it with the **TinyWorld** icon in the system tray:

| Item | What it does |
|---|---|
| **Pause** | Pets stand still and the mouse passes through them; click again to bring them back to life |
| **Quiet** | For 30 minutes, 1 hour or until you turn it off: pets say nothing (chit-chat and reminders are skipped, weather, event and update notices wait until Quiet ends) and walk around much less. Click the ticked choice again to turn it off; it also ends on its own when the time is up |
| **Show / hide pets** | Hides the pets completely (the app stops its loop and uses no CPU) |
| **Settings…** | **Pets** tab: pick 1–3 characters, pet size (50–200%), walking speed, chat with pets (off by default), language (same as Windows, Vietnamese, English), start with Windows. **World** tab: city (real weather, local time there), toggles for weather around pets, the temperature label next to pets, chit-chat, the event calendar and the ghost at 2 a.m.; the Preview section lets pets meet a kind of weather right away (sunny, clear, cloudy, rain, storm, snow, fog, petals) and a temperature (hot 36°C, cold 8°C) for 30 seconds, or the ghost; the active button lights up with a countdown, click it again to stop; edit the event calendar. **Health** tab (all off by default): screen time today and over 7 days, break reminders, water reminders, bedtime reminders, Ctrl+S spam. Changes apply right away. The running version is shown under the title; **Open log folder** at the bottom opens the folder with `tinyworld.log` |
| **Quit** | Saves the pets' state, then quits |
| **DevTools** | Dev only (the overlay lets the mouse pass through, so F12 can't reach it) |

While running in dev, `window.__tinyworld` in DevTools lets you inspect and tweak the pets (`pet` is the
first one, `members` the whole group), e.g. `__tinyworld.world.pets.forEach((p) => (p.sinceInteraction = 1e6))`
sends the whole group to sleep right away. Turning on **Start with Windows** while running in dev records
the path of the debug exe, so remember to turn it off again.

Interacting with the pets:

| Action | Pet |
|---|---|
| Leave it alone | Stands, walks and runs on its own; turns around at the screen edge. With a single pet, an edge that borders another screen sometimes leads it over there; with 2 or more, the group stays on one screen. Pets walk through each other |
| A window nearby | Climbs the side of the window up to its top edge (for a floating window it jumps up to grab the side, then climbs; it gives up if the bottom is more than about 270 px above where it stands), walks along it, perches on the edge, jumps to another window, or jumps or climbs down |
| Drag another window right up to a pet | The pet runs away the other way; on a window with nowhere left to go it gets down, and on the ground when cornered against the screen edge it jumps in fright. Maximizing or resizing a window doesn't count |
| Close a window nearby (within about 400 px) | The pet turns toward it and hops twice to celebrate (not every time; at most once every 15 seconds) |
| Drag the window a pet is standing on | The pet rides along; drag it up against the top of the screen and there's no room left to stand, so the pet falls |
| Minimize or close that window | The pet falls onto the window below or the taskbar |
| Another window covers it | The part of the pet under that window is hidden (clicks there go to the window), and after a while it walks out to where it isn't covered. When it's fully covered (e.g. you click a maximized window underneath) it drops in front of every window right away, letting go if it was climbing |
| Cursor nearby | When standing still, it turns toward the cursor |
| Click | Hops once, then keeps walking or running; wakes up if asleep; lets go if climbing. Rapid clicks only make it hop once |
| Pick it up and drop it | Falls to the edge of the window below or the taskbar and bounces a little on landing; dropped from high up it gets dizzy: wobbling, with stars circling its head |
| Drag, fling the mouse and let go | Gets thrown with momentum and bounces off walls; thrown toward another screen it flies over there |
| Drag a pet to another screen | The pet follows the cursor onto that screen |
| With several pets, throw or drag one to another screen | The others run after it, coming in from the edge facing the old screen: the closest to that edge first, the farthest last; sleeping ones lie down right by that edge |
| Click where two pets overlap | The one on top gets the click; the one just clicked or dragged goes on top of the others |
| Double-click | That pet says the time, weekday, date, lunar date and the weather in the chosen city ("No internet :))" when offline) |
| Right-click | Opens a chat window with that pet right next to it (turn on **Chat with pets** in Settings first); the pet stands still and waits until the chat closes. Ask quick questions about exchange rates, news, unit conversions... through Google Gemini, no account needed; the suggested questions change every day. Links in answers are clickable, and there's a Copy button. Ask for a picture and the pictures Google finds show under the answer; click one to open the page it comes from. The app stops you from sending too fast (one message every 3 seconds, 30 an hour); when offline it shows "No internet". If Google takes too long to answer mid-chat, the pet says the chat got too long and starts over, without the earlier turns |
| Two pets meet | They stop and face each other, sometimes say hello or walk together for a while; at bedtime they sleep side by side |
| No pet touched for 3 minutes | The whole group comes down to the taskbar (jumping or climbing down from windows) and falls asleep one by one: clicking or dragging any pet counts for the whole group. Clicking anywhere on the screen or dragging a pet wakes everyone up |
| Hold **Ctrl** while clicking | The click goes through the pet to the app underneath |
| Hold **Shift** while clicking Long | Long lights up a fire aura for a few seconds. With other pets around, Long transforms, then blinks next to each one in turn and whips it once with its tail; a pet that gets hit jumps in fright, vanishes for 5 seconds and reappears in the same spot. If a pet is being dragged, climbing, jumping or chatting, Long waits a while and then skips it; picking Long up midway stops the combo. Shift + click on any other pet counts as a normal click |

If you change the resolution, DPI scale, taskbar position or size, or plug in or unplug a monitor while the
app is running, the overlay readjusts within about a second: pets stand on the new ground, a pet standing
on a window stays on that window's edge, and if the monitor a pet is on gets unplugged, the pet moves to
the main screen. With an auto-hiding taskbar, pets stand on top of it when it slides up and drop to the
bottom of the screen when it slides away. The app only supports 100% scaling.

Clicking a pet doesn't take focus away from the app you're using. When an app goes fullscreen (videos,
games, presentations: a window covering the entire screen, not counting maximized windows), the pets hide
and come back when fullscreen ends. While hidden (including when hidden from the tray) the app stops its
loop entirely, and the pets wait where they were until they reappear.

Things that happen without you doing anything (each can be turned off in the **World** tab):

- At night (from sunset in the chosen city, or 19:00–6:00 without a city) pets walk slower and get sleepy
  sooner
- Weather only around the pets, fitted to each one's shape: sunny (sunbeams, sparkles; falling petals in
  spring), clear night (stars, the odd shooting star), cloudy (clouds drifting overhead), rain (drops
  splashing on the ground), snow, fog, thunder (flashes, pets get startled). From 33°C heat haze rises on
  both sides of a pet; at 15°C and below pets breathe out puffs of mist (puffing harder when walking or
  running). Without a city the weather is simulated by season, with no temperature; with a city but no
  internet, one pet says "No internet :))" and there are no effects
- Temperature label next to the pets (needs a city): a weather icon and °C, standing behind one pet. When
  the weather changes, the pet's line includes the temperature, e.g. "It's raining, 24°C :("
- Chit-chat (greeting each other, grumbling about the rain): one line every 15 minutes for the whole group
- Event calendar: on the day (Tết, Mid-Autumn, Christmas... or events you add, on the solar or lunar
  calendar) one pet says that occasion's line once a day. The World tab shows today's events
- From 2:00 to 2:30 a.m., if you're at the computer (a key press or mouse input within the last minute), a
  ghost flies around the screen once a night; it swoops down to each pet's height, and any pet it flies
  past jumps in fright, waking up even if asleep

**Health** tab (all off by default, the data stays on this computer):

- Screen time: counted while you touch the mouse or keyboard; more than 5 minutes away counts as a break.
  Shows today, the current stretch, the longest stretch, how many breaks you took and the last 7 days. Can
  be cleared
- After 50 minutes in a row (adjustable) a pet hops up to remind you to rest your eyes; every 60 minutes
  at the computer (adjustable) it reminds you to drink water; past bedtime (23:00 by default) while you're
  still at the computer it reminds you to go to bed every 30 minutes. Click the pet that's reminding you to
  dismiss it
- Press Ctrl+S 5 times in 10 seconds and a pet complains "It's already saved :(((("
- While you're typing (input while the mouse stands still), pets don't chit-chat

## Building a release

```bash
pnpm icons:desktop   # regenerate the icons if you just replaced assets/icon.png
pnpm build:desktop   # NSIS installer in desktop/src-tauri/target/release/bundle/nsis
```

The source logo is [`assets/icon.png`](assets/icon.png), shared by the app, the system tray, the installer
and the portable exe (how to replace it: [assets/README.md](assets/README.md#thay-logo)). The Build and
Release workflows on GitHub generate the icon set from this logo before compiling.

### Automatic builds on push to `main` or `test`

Every push to `main` or `test` runs the [`.github/workflows/build.yml`](.github/workflows/build.yml)
workflow: typecheck, tests, `cargo test`, then a Windows `.exe` build (about 10–15 minutes the first time,
faster afterwards thanks to the cache). The `test` branch is for downloading artifacts to try out before
merging into `main`; this workflow doesn't create a GitHub Release.

To download: GitHub → **Actions** tab → click the latest run → **Artifacts** at the bottom of the page. Each
file is its own artifact and downloads as the `.exe` itself, not a zip (kept for 30 days). Both files used
to share one zip, and Explorer shows an exe inside a zip with the generic exe icon, which made the app
look like it had lost its icon.

| File | Use it to |
|---|---|
| `TinyWorld_<version>-<run number>-<commit>_x64-setup.exe` | Install the app (with shortcuts, uninstallable from Settings) |
| `TinyWorld_<version>-<run number>-<commit>_x64-portable.exe` | Run it directly without installing (needs WebView2, preinstalled on recent Windows 10/11) |

`<version>` is the upcoming version: if the commit has already been released, that exact version;
otherwise the latest release with its last number bumped (`1.3.3` if `v1.3.2` exists). This version is
also baked into the app; the Settings window shows it with the run number and commit (e.g.
`Version 1.3.3 · build 57 · 14e831c`) to tell it apart from the Release of the same version. In dev it
shows `Dev build`.

If the tests fail, no files are built. Pushing repeatedly cancels the older build and keeps only the
newest. It only builds when a push changes something that goes into the app: `desktop/`, `packages/`,
`assets/` (sprites, icons), `scripts/`, dependencies. A push that only touches `.md` files or the landing
page (`website/`) doesn't build. To rebuild without pushing: **Actions → Build → Run workflow**.

### Releasing with GitHub Actions (manual)

To publish an official release on the **Releases** page, the
[`.github/workflows/release.yml`](.github/workflows/release.yml) workflow runs the tests, builds
`TinyWorld_<version>_x64-setup.exe` and `TinyWorld_<version>_x64-portable.exe`, and attaches both to the
GitHub Release `v<version>`.

1. Push the code you want to release to GitHub.
2. GitHub → **Actions → Release → Run workflow** → enter the **version** (`x.y.z`, e.g. `0.2.0`).
   The version is baked into the app at build time; no file in the repo needs editing.
3. It creates a **draft** by default; check the files on the **Releases** page, then click **Publish
   release**. Untick *draft* to publish right away.

Running it again with a version that's already *published* is rejected (bump the version); running it
again while it's still a draft overwrites the old files. The app isn't code-signed, so Windows shows a
SmartScreen warning: choose **More info → Run anyway**.

### In-app updates

The app ([`update.rs`](desktop/src-tauri/src/update.rs)) asks GitHub for the repo's `releases/latest` 20
seconds after starting and then every 6 hours; the **Check for updates** button under the version line in
Settings asks right away. If there's a newer version (compared by `x.y.z`) with a `*-setup.exe` file,
Settings shows a **Version x.y.z is available** card and one pet says a line (once per version). Clicking
**Update now** downloads the installer to `%TEMP%\TinyWorld-update`, checks its size and its SHA-256 against
the `digest` GitHub lists for the file (catches a damaged or swapped download; it's no substitute for code
signing; assets uploaded before GitHub had digests are only size-checked), quits
the app as if you clicked Quit, then runs the installer with `/P /UPDATE /R` (progress only, installs over
the existing install folder, then reopens the app). A portable copy that updates becomes an installed
copy; the NSIS hook closes the running copy and points "Start with Windows" at the newly installed one.

- Only **published** releases are offered: drafts and pre-releases aren't in `releases/latest`.
- Version 1.5.0 and earlier have no updater: users have to install the first version with an updater over
  them by hand, once.
- Double-clicking an installer newer than the installed version goes through the same flow: the hook in
  [`hooks.nsh`](desktop/src-tauri/windows/hooks.nsh) reruns the installer with `/P /UPDATE /R`, skipping
  the "Already Installed" page (that page preselects uninstalling the old version first, and some machines
  report "Unable to uninstall!"). The 1.6.0 installer doesn't have this hook yet.
- Builds from `main`/`test` have a version ≥ the latest release, so they're never offered an "update" back
  to an older version.
- Dev runs don't check on their own (set `TINYWORLD_CHECK_UPDATE` to enable it) and don't install
  updates, because the installer would overwrite the real installed copy on the machine (set
  `TINYWORLD_INSTALL_UPDATE` to try the whole flow).
- To test checking and downloading against the real GitHub: `cargo test -- --ignored github_that` in
  `desktop/src-tauri`.

## Landing page

[`website/`](website/) is a static landing page (Vietnamese at `/`, English at `/en/`): the app's real
pets living on a miniature screen, a weather demo, the character lineup and a button to download the
latest version. Editing the text and rebuilding it (`pnpm prepare:website`): [website/README.md](website/README.md).

## Architecture

```
Rust (cursor.rs) --GetCursorPos ~60 times/s--> event "cursor-moved" (overlay CSS pixels)
                                                      |
                                                      v
                      overlay: is the cursor over a visible part of a pet? (by sprite alpha)
                                                      |
                                invoke("set_click_through", { enabled })  (only when it changes)
                                                      |
                                                      v
                      Rust: turn mouse pass-through on/off for the overlay (set_ignore_cursor_events)
```

- **Windows** (`src-tauri/src/window_list.rs`): Windows reports whenever a window opens, closes, moves or
  changes stacking order (`SetWinEventHook`); a background thread batches these, reads the window list at
  most 30 times a second and only sends a `windows-changed` event when something differs, sleeping when
  nothing changes (it rereads every 2 seconds in case something was missed). It only takes the visible
  frame (DWM, without the transparent border), the stacking order and the class name to skip the desktop
  and taskbar; it never reads titles or content. Minimized and hidden windows, windows on another virtual
  desktop, tool windows and click-through windows are skipped. Windows also reports cursor movement (the
  same event as a window moving), so while the pets sleep it only tracks movement of explorer's windows
  (the taskbar) and rereads every 10 seconds; while the overlay is hidden (fullscreen app) the hook is
  removed entirely.
- **Terrain** (`packages/sim/src/terrain.ts`): a window's top edge is ground to stand on and its sides are
  walls to climb, except where a window on top covers them. A pet standing on the taskbar or in the air is
  in front of every window; for a pet standing on or climbing a window, the part covered by windows on top
  isn't drawn (it's erased on the canvas) and doesn't catch the mouse.
- **Overlay** (`src-tauri/src/overlay.rs`): a transparent, borderless, always-on-top window with no
  taskbar button, covering the work area (minus the taskbar) of one screen, the main screen at startup. It
  doesn't cover the whole screen because Windows would then treat it as a fullscreen app: notifications
  would be muted, and the pets would keep hiding and reappearing. Coordinates sent to the frontend are all
  overlay CSS pixels; Rust converts them from physical pixels by DPI.
- **Multiple monitors**: `screen_info` includes the work areas of the other screens (`neighbors`), so pets
  know which edges border another screen. When a pet is dragged, thrown or walks off an edge, the frontend
  calls `move_overlay` with a point just past that edge; if the point is on another screen, Rust moves the
  overlay there, then sends a `screen-changed` event with how to map old coordinates to new ones
  (`remap`). Rust re-measures the screens every second; if the resolution, DPI or taskbar changes or a
  monitor is unplugged, it resets the overlay and sends the same event.
- **Cursor** (`src-tauri/src/cursor.rs`): Rust reads the cursor about 60 times a second but only sends
  `cursor-moved` while the cursor is inside an area around a pet that the frontend reported
  (`set_cursor_interest`, one area per pet: within 300 px of sight for an awake pet, just around the body
  for a sleeping one), right when it leaves that area, on mouse down/up (a click anywhere wakes sleeping
  pets) and when an area changes. That way, moving the mouse far from the pets doesn't wake the WebView.
  Pets only use the cursor position to turn their heads toward it and to switch click-through on and off.
- **Auto-hiding taskbar**: when it slides up, the taskbar covers the bottom edge of the overlay. The
  window-tracking thread reads the taskbar frame together with the window list and sends its top edge
  (`taskbarTop` in `windows-changed`); if that edge is above the bottom of the work area, the frontend uses
  it as the ground.
- **Pets**: each pet is a small `<canvas>` the size of one frame, moved with CSS transforms and only
  redrawn when its frame changes. The whole group (up to 3 pets) shares one overlay and one loop; only the
  sprites of the characters on screen are loaded, and removed characters are dropped. The loop runs at
  most 30 fps, setting a timer before asking for `requestAnimationFrame` (asking for rAF continuously wakes
  the WebView at the display's refresh rate, 60–144 times a second). When the whole group stands still or
  perches, it only wakes when a pet changes frame (about 7 times a second); when the whole group sleeps it
  stops entirely and Rust tells WebView2 to use less RAM (`MemoryUsageTargetLevel`), and the same happens
  while the overlay is hidden. The alpha mask used for hit-testing stores 1 bit per pixel.
- **Logic** (`packages/sim`): no dependency on the DOM or Tauri, tested with vitest. A fixed time step
  keeps behavior independent of fps; a seeded RNG makes random behavior testable.
- **Weather** (`src-tauri/src/weather.rs`): only Rust goes online, never the webview. With a city chosen,
  it asks Open-Meteo at startup, when the city changes and then every 30 minutes, and sends a
  `weather-changed` event; on errors it waits 1 minute, doubling each time up to 30 minutes, and only sends
  `weather-failed` on the first error. City search (`search_city`) also goes through Rust. The overlay
  (`src/overlay/ambience.ts`) turns the weather into effects, and handles night time, holiday lines and the
  ghost by the time in the chosen city (`wallClock`, IANA time zone); the lunar calendar is computed
  locally (`packages/sim/src/lunar.ts`). Every pet line lives in `packages/sim/src/lines.ts`.
- **Health** (`src-tauri/src/activity.rs`): a background thread reads the time of the last input
  (`GetLastInputInfo`) and the cursor position every second, without a keyboard hook. `Tracker` (pure
  logic, tested) adds up screen time from the gaps between inputs, decides on break, water and late-night
  reminders, and sends a `reminder` event; input while the mouse stands still sends `activity-changed`
  (typing). Ctrl+S spam is read in the cursor loop of `cursor.rs`. The overlay only makes the pets talk
  (`Ambience.remind`).
- **Chat** (`src-tauri/src/chat.rs`, `src-tauri/src/gemini.rs`, `src/chat/`): a separate React window
  (`chat.html`), only created when you right-click a pet and destroyed when it closes (Rust sends
  `chat-closed` to the overlay so the chatting pet walks again). The chat window builds the message to send
  (the character's personality, the local time, the last 6 turns, `prompt.ts`); Rust checks the send
  limits (`Limiter`), then calls the endpoint gemini.google.com uses when signed out (`gemini.rs`: sends
  only the question, no headers, no browser impersonation, no page scraping; Google picks the model). No
  server, no open ports. Markdown answers are parsed into blocks (`markdown.ts`) and drawn with React,
  never as raw HTML.
- **Language** (`packages/core/src/i18n/`): all user-facing text is in `vi.json` and `en.json`, shared by
  the frontend (Settings, the chat window, pet lines via `packages/sim/src/lines.ts`) and Rust (`i18n.rs`
  embeds both files at build time: tray menu, chat window title, error messages). `{name}` placeholders are
  filled in when used. Add new text to both files; a missing entry fails typecheck and tests. By default
  the app follows the Windows display language (Vietnamese for Vietnamese, English otherwise); Rust works
  out `lang` and sends it with the settings. Built-in calendar events whose name and line haven't been
  edited (`preset`) show in the current language.
- **Settings window** (`src/settings/`, `src-tauri/src/settings.rs`): a separate React page
  (`settings.html`), only created when you click **Settings…** and destroyed when it closes to save RAM.
  The overlay doesn't pull in React.
- **Data** lives in `%APPDATA%\com.tinyworld.app\`, written to a temp file and then renamed:
  - `world.json`: each pet's position, facing and whether it's asleep (the id is the pack folder name).
    Saved every 30 seconds (only when something changed) and on **Quit**; an abrupt shutdown loses at most
    30 seconds.
  - `settings.json`: the characters on screen (`pets`, pack folder names in the order they were picked),
    pet size, speed, language (`language`: `auto`, `vi`, `en`), city (name, coordinates, time zone), the
    World tab toggles and the event calendar (`occasions`, at most 30 events). If it's edited by hand with
    bad values, the app clamps them to the allowed range, drops duplicate names, drops pets beyond 3 and
    events on dates that don't exist; missing packs are skipped, and if none are left the first pack is
    used. Files from older versions (`pet`, a single character; no World tab entries yet) still load.
  - `weather.json`: the last weather fetched, so starting the app offline can still use it if it's less
    than 3 hours old.
  - `stats.json`: per day for the last 30 days: screen time, the longest stretch (ms) and the number of
    breaks; only there when screen time tracking is on. Written every minute (when something changed) and
    on quit; the clear button in the Health tab overwrites it with empty data.
  - `tinyworld.log`: errors from Rust (chat, weather, updates, Win32) and panics, one timestamped line
    each, local time. Release builds have no console, so this is the only place they end up. Over 1 MB it is
    renamed to `tinyworld.old.log` and a new file starts. It never leaves the computer.
  - **Start with Windows** isn't stored here but as the `TinyWorld` value in
    `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (pointing to the running exe).

## Testing

```bash
pnpm typecheck
pnpm test                               # sprite packs, Long's effects, overlay drag/double-click, Long's combo, pet.json, vi/en text files, FSM, World/Pet, pet groups and Quiet, window terrain, cursor, screen changes, world.json, weather, lunar calendar, event calendar
cd desktop/src-tauri && cargo test      # DPI coordinates, neighboring screens, coordinate mapping between screens, window list, saved state, settings, language, event calendar, Open-Meteo parsing, screen time and reminders, Ctrl+S spam, chat send limits, chat window position, Gemini reply parsing, suggested questions, chat images, update parsing and installer SHA-256, log file, registry
```

`pnpm test` includes `pnpm test:sprites` (the sprite normalization script, `prepare-sprites.mjs --check`
on all 22 packs and `prepare-aura.mjs --check` for Long's effects), `pnpm test:overlay` (drag and
double-click on the overlay, Long's tail-whip combo, chat markdown and message building) and the vitest
suites in `packages/*`.

To measure CPU and RAM over a long run (leak check) or on battery, start the app and run
`powershell -ExecutionPolicy Bypass -File scripts/measure.ps1 -Minutes 480 -Every 60`: it adds up
`tinyworld.exe` and its WebView2 processes, writes one CSV row per sample (with AC/battery and charge) and
prints a summary at the end.
