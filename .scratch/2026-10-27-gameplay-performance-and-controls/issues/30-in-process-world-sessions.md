# 30: W1: In-process World sessions

Status: needs-triage
Blocked by: 27

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: W1; keep this separate from other PR scopes.

## What to build

Replace W0's process re-exec with in-process switching between the World list
and a World, as Minecraft (integrated server) and Luanti (per-session client
and server objects) do. Scope every Server and Client resource, Startup
system, task and Mod instance to a World session, likely via Bevy `States`,
so that leaving a World tears it down completely and opening another builds
it fresh in the same window.

Show the loading screen at once: move Mod loading and save validation behind
the window, so opening a World never leaves the screen empty. Ticket 27
moved this here (2026-10-29). The user asked (2026-10-29) for one window that
never closes when a World opens, with a loading bar that follows the real
stages: reading the save, loading the Game, then the first terrain.

This is the design the spec's World-switch rule assumes: session-only undo,
measurement and pending input are cleared, and old-World jobs or answers
cannot land in a newly opened World with coincident coordinates.

## Notes

Raised during ticket 27's 2026-10-29 grilling (Q19). On main 96ea33e the
Server and Client use 17 `Startup` systems, about 329 resource inits and no
`States`, and Bevy allows one window event loop per process. The refactor
touches nearly every plugin, so schedule it when no parallel worktree is
editing Server or Client plugins. Needs grilling before implementation.

Ticket 32 carries the window position and fullscreen over re-exec; that code
is what this ticket deletes, so keep 32 minimal rather than reusable.
