# 39: Pick, pin, then build with G or cancel with K or Backspace

Status: ready-for-agent

Blocked by: 29, 34

Spec: [Phase 20](../spec.md).

## What to build

Play-testing after ticket 34: "should be able to cancel the place tool with
either k or backspace. when a template is selected should be able to move the
preview template around and press right click place the preview. come close
you will see press g to build the preview and key k/backspace to cancel the
preview".

- **Pick:** choosing a Template in the list closes it into play, and the ghost
  follows the aim.
- **Pin:** right mouse pins the ghost where it is aimed; with one pinned it
  does nothing more.
- **Build or cancel:** within 8 m of the pinned preview, a prompt under the
  reticle reads `G Build · K / Backspace Cancel`. Use (G) builds it there; K or
  Backspace cancels it, from anywhere.
- **Put the Tool away:** holding the place Tool with no preview pinned, K or
  Backspace puts it away. K with any other hand opens the list as before.
- The mockup changes first, then the Client.

## Acceptance criteria

- [ ] The mockup's keys, card and prompt follow the flow above.
- [ ] Picking a Template closes the list, tested.
- [ ] Right mouse pins and sends nothing; G within 8 m of the pinned preview
      sends the pinned intent; G further away sends nothing, tested.
- [ ] K and Backspace cancel a pinned preview and keep the Tool; without one
      they put the Tool away, tested.
- [ ] The prompt shows only near a pinned preview, tested.

## Comments
