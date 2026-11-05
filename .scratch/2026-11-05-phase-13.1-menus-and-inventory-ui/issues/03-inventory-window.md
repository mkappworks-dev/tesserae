# 03: Inventory window

Status: done

Blocked by: 02

Spec: [Phase 13.1](../spec.md).

## What to build

I opens the Inventory over the dimmed, blurred World: the backpack, the hotbar beneath it, and an item details panel (name, source, the Material it places, count, stack limit). Click a stack then a slot to move, merge or swap; drag and drop does the same. Shift-click moves a stack between backpack and hotbar. Split halves a stack into an empty backpack slot. Sort merges and orders the backpack. The backpack header reads "Full" when every slot is used. Every change is an intent the Server validates and answers with the new Inventory or a refusal. I or Escape closes the window. The look follows the prototype's Inventory overlay.

## Acceptance criteria

- [x] Move, merge, swap, split, Sort and Shift-click each work through a Server intent and survive save and reopen.
- [x] An impossible change (a split with no empty slot, a merge into a full stack) is refused with a message and changes nothing.
- [x] Opening the window frees the cursor; closing it returns to play.
- [x] Client App tests open the window with I and drive a move and a Shift-click.
