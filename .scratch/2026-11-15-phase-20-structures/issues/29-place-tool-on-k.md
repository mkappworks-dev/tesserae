# 29: K opens and closes Structure placement

Status: ready-for-agent

Blocked by: 10, 11

Spec: [Phase 20](../spec.md).

## What to build

Play-testing asked for Structure placement on its own key: "move the structure
placement to K. also pressing backspace/K should get me out of structure
placement overlay".

- K asks to use the place Tool, held or not, and the Server's answer opens its
  Template list with the place Tool in hand, as L opens the Layout menu.
  Survival hears the same refusal as before.
- With the list open, K closes it back into play with the Tool in hand;
  Backspace closes it and puts the Tool away; Escape still closes it with the
  cursor free.
- Use (G) no longer opens the list.
- Check K against every other binding, in the defaults and in the controls
  docs.
- The mockup changes first: its key handler and its hints (`K Templates` on
  the HUD card, `K opens this list` in the footer).

## Acceptance criteria

- [ ] The mockup's place Tool opens on K and closes on K or Backspace; its
      hints say so.
- [ ] K opens the list from play, held or not, tested through the Client.
- [ ] K and Backspace close it, tested; Backspace puts the Tool away.
- [ ] G with the place Tool in hand opens nothing.
- [ ] No other default binding uses K.

## Comments
