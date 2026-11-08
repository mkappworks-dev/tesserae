# Green ambience, menu noise and Music controls

The user identified the tonal “ooo” as green-area ambience: it persists with
Music at 0. The old bird-and-synth outdoor recording is replaced by two CC0
wind/rustle field recordings with a 4 s internal transition and loop seam,
filtered to 80–1800 Hz at a -34 LUFS authoring target. The resulting bed is
104.59 s, rather than a padded short phrase. Sources and edits are listed in
`games/terra/CREDITS.md`. The old bird-containing music track is no longer
declared or selected. Terra currently selects the remaining complete Hypnotic
Chill track; the Core's multi-track non-repeat behavior remains available.
Animal vocalizations are reserved for future nearby animal entities, using
positional Server-driven events. No animals or animal sounds are added here.

World playback previously continued behind Take a moment. Menu, Settings and
save modals now fade playback to zero over 0.1 s on the real clock. Background
AudioSinks pause; the music scheduler does not advance. Old one-shots are
removed after fading, and newly received action events are consumed rather
than queued for resume. Background sinks resume at their retained position;
the World simulation itself is unchanged.

Tests exercise action-event dropping and clean resume, music silence timer
freeze, existing track identity across menu entry/resume, Settings silence even
with virtual time paused, and the actual Music control's ValueChange → Apply
path. Music 0 produces zero gain on an already-existing track; Music 100
restores it after resume while Ambient remains independently audible.

1,165 Engine tests pass, with one existing ignored test. Demo WASM builds,
Clippy with warnings denied, rustfmt, prototype JavaScript syntax and Terra
validation pass. These are automated checks, not a claim of headphone/speaker
listening or that an OS/driver audio fault has been reproduced. Tickets 09 and
13 retain the human listening acceptance. No commit or push.
