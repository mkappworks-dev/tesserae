# Worlds open in-process as sessions

The World list and every World share one process, one window and one `App`. An
`AppState` (`Boot`, `List`, `Loading`, `InWorld`, `Closing`) brackets a World
session; plugins register through `SessionApp` so that a session's resources,
entities and messages exist only while it does. Leaving a World removes whatever
was added beyond the baseline recorded before it began, so the next World is
built fresh.

The first design re-executed the process per World, which kept old-World jobs
out of a new World by construction but closed the window, repeated start-up and
could show nothing until Mod loading finished. Minecraft (integrated server) and
Luanti (per-session objects) switch in-process. Bevy allows one window event
loop per process, which rules out a second `App` per World.

## Consequences

A resource the World list also reads (input ownership, settings, the menu) is
reset rather than removed, with `init_session_state`. Mod instances and the save
task drop in `SessionTeardown`, before the writer lock is released. Loading runs
off the main thread and reports its stage, so the screen shows at once. A
session leak is a regression: `TESSERAE_SOAK_CYCLES=n` (dev) opens and leaves a
World n times and logs entity, resource and asset counts, which must not grow.
