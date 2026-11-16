# 29: K opens and closes Structure placement

Status: done

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

- [x] The mockup's place Tool opens on K and closes on K or Backspace; its
      hints say so.
- [x] K opens the list from play, held or not, tested through the Client.
- [x] K and Backspace close it, tested; Backspace puts the Tool away.
- [x] G with the place Tool in hand opens nothing.
- [x] No other default binding uses K.

## Comments

Mockup: `structures.js`'s key handler takes K from play (it puts the place Tool
in hand and opens the list), K or Backspace closes it (Backspace puts the Tool
away), Escape still closes it; the HUD card says `K Templates` and the footer
`K opens and closes this list · Backspace puts the Tool away`; the bindings list
in `settings.js` gains "Structure placement: K". Checked in a browser.

Client: a new `Action::Structures`, default K, labelled "Structure placement"
under Building. `structure_menu::toggle` sends the place Tool's Use on K from
play whether it is held or not and notes it asked; `open_on_use` then opens the
list on the Server's answer and puts the Tool in hand. With the list open, K
closes it into play, the clear key closes it and empties the hand. `intercept`
now takes Use too, so G with the place Tool held sends nothing; the card's and
the list's key hints read the Structures and clear bindings.

Key check: no default binding used K. A saved profile that binds K to
something else keeps it, and Structure placement is then unbound there, as the
profile merge already drops a new default that collides. The play-tested
profile does not use K. Docs: `docs/controls.md` and the authoring guide.
