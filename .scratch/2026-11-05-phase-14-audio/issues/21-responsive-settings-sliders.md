# 21: Responsive Settings sliders

Status: done

All Settings sliders must accept clicks and continuous drags. Decorative thumbs
must not intercept pointer presses that establish the slider's pressed state;
the page must retain the dragged entity until release. Track clicks should snap
to the pointer. Preserve Apply/Discard and quiet-menu audio behavior.

## Acceptance criteria

- [x] Real pointer presses and drags update slider values across frames.
- [x] All Settings sliders share the correction; draft behavior is preserved.
- [x] Settings tests, lint and formatting pass.

## Comments

- Shared slider thumbs ignore picking, so the slider itself receives the press
  and holds Pressed until release; Settings rendering therefore preserves the
  live entity throughout dragging. Track clicks snap to the pointer position.
- A regression builds the Audio page and uses Bevy pointer press/drag events
  across frames, checks both endpoints and release, and verifies draft-only
  editing followed by Apply. All 22 Settings tests pass, including the existing
  sensitivity and Apply/Discard tests. All-features Clippy and formatting pass.
- This is an interaction correction to existing controls; audio preview and
  Apply/Discard semantics remain unchanged. No Game assets or WIT changes.
