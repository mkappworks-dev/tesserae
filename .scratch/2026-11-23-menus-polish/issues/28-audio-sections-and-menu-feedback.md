# 28: Audio sections, Interface volume and menu rumble

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

As in the approved mockup (`/settings/audio`, `/settings/controller`, commit
84c5929a):

- Settings › Audio › Sound mix gains an **Interface** slider (default 100, 0
  mutes) that sets the menu click's loudness on its own, no longer through
  Effects. Reset on the Audio page resets it.
- The collapsible "Fine-tune sound effects" becomes an always-visible
  **Sound effects** section, headed like Sound mix, with the note "Each is
  relative to Effects; 0 mutes it." and every effect slider (Footsteps,
  Landings, Breaking & striking, Placing, Eating, Swimming, Floating water,
  Pickups, Tools, Hurt & death). No expand or collapse.
- Settings › Controller gains **Rumble on menu press** (On/Off, default On),
  which turns off ticket 17's menu-press rumble.

## Acceptance criteria

- [x] The menu click follows Interface, not Effects; 0 mutes it
- [x] Sound effects is always shown with every effect slider
- [x] Rumble on menu press Off stops the menu-press rumble
- [x] Both new settings are saved and restored

## Comments

- Master volume still scales the click: Interface sits under Master like the
  other categories. `Volumes::interface_gain` lives in tesserae-core beside
  `gain`, not as a new `SoundCategory`, so Game sound libraries cannot claim it.
- The Sound effects heading's note is a new `settings::widgets::noted_group`.
