# 40: G or Backspace closes a chest or workbench

Status: done

Spec: [Phase 20](../spec.md).

## What to build

Play-testing: "when you press g for chest, workbench, click on G or backspace
should close it". Use (G) on a chest or a Station opens the Inventory window
with its panel; it closed only with the Inventory key (I) or Escape.

- While the Inventory window is open, Use (G) or the clear key (Backspace)
  closes it into play, the cursor grabbed, as the Inventory key does; typing in
  the Catalogue search keeps them as text.
- The window's title shows the three keys that close it.
- The mockup changes first, then the Client.

## Acceptance criteria

- [x] The mockup's storage screen closes on G or Backspace and shows the keys.
- [x] The Client's window closes into play on G or Backspace, tested.

## Comments

Mockup: `inventory.js`'s key handler closes the storage screen on Escape,
Backspace or G (it already ignores keys while typing); the title shows `I`,
`G` and `⌫`. Client: `inventory_window::toggle` closes on Use or the clear key
while the window is open, taking both so they do nothing else; it already
returns early during text entry. The title draws the first binding of the
Inventory, Use and clear actions. Test:
`use_or_the_clear_key_closes_the_window_into_play`.
