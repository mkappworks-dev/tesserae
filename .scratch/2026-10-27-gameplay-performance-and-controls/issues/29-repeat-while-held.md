# 29: Optional repeat place/remove while held

Status: done
Blocked by: none (07 merged as PR #28)

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: its own PR, built on ticket 07; Client-only.

## What to build

Add one device-neutral behaviour preference, "Repeat while held", default
off, on the Behavior settings page. Store it in `Preferences`, so keyboard
and controller share it. It applies only in hold mode; timed-tap removal
stays one per tap. With it off, behaviour is unchanged.

### Removal

- While Remove is held and one removal completes, start a fresh
  Material-timed removal on the next Server-aimed target. Progress never
  carries over, and every removal takes its full duration.
- Releasing the button, losing the target or reach, or a menu, focus or
  device change stops the chain.
- Changing hand, slot, Shape or orientation, sampling, or undo/redo stops the
  chain until a fresh press.

### Placement

- A held Place sends its first intent on press. After 0.25 s it sends another
  every 0.2 s, aimed fresh each time (Minecraft Java's four ticks).
- The Client paces this; each repeat is an ordinary intent the Server
  validates as today. The Server's existing per-tick and pending caps bound
  throughput.
- Only solids repeat. Fluid placement and ignite stay one per press.
- A refused repeat is silent and the chain keeps trying. Only a hold's first
  press shows the usual alert toast.

### Not included

- No Server, WIT or core change.
- Each repeated edit is its own history entry; grouping a hold into one undo
  step is deferred.
- No per-device split of the preference.

## Acceptance criteria

- [x] Edits per held second are identical at 60/144/1000 Hz render updates.
- [x] No repeat bypasses Material duration or transfers progress.
- [x] Fluids and fire never repeat; refused repeats raise no toasts.
- [x] Every cancellation route above stops the chain.
- [x] With the preference off, behaviour matches the existing tests exactly.

## Validation and authorization

Use behaviour/intent regressions and the spec's build/review requirements.
Preserve Server guards and count bounds. Local implementation is authorized
by this status once ticket 07 is done; push, publication and merge need
separate user instruction.

## Comments

### 2026-10-29: recorded during controller grilling

Raised in ticket 07's Q27. Out of scope for the controller PR by user
decision.

### 2026-10-29: triaged and grilled

The user accepted recommendations Q1-Q8. The draft's "Server-paced" wording
is replaced by Client pacing within the Server's existing caps. The spec's
one-removal-per-activation rule now has this opt-in exception.
