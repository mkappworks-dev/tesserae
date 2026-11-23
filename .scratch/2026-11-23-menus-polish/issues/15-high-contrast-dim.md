# 15: High-contrast dim is clearly darker

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

With the accessibility high-contrast setting on, overlays and dialogs dim the scenery clearly more than the normal dim from ticket 09, using the same `theme::css_dim` conversion.

## Acceptance criteria

- [x] High-contrast dim is visibly darker than the normal dim, checked by capture
- [x] A theme test pins the high-contrast dim above the normal one by a clear margin

## Comments

The mockup has no stronger dim, so `theme::overlay()` under stronger contrast is CSS 0.85 through `css_dim` (normal stays 0.70): mid-grey dims to 0.11 instead of 0.18. Only the dialog overlay changes; the launcher/World `shade()`, the pause shade and the sheet shade are the same in both modes.
