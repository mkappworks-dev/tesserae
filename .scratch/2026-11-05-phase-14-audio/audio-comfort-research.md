# Audio comfort research and proposed revision

Status: ready-for-human

Research snapshot before implementation. Implemented results and remaining
listening checks are in [comfort-validation.md](comfort-validation.md).

## Baseline before the approved comfort update

Measured with local ffprobe, not inferred from filenames: temperate 30.0 s,
dry 5.96 s, cold 8.11 s, cave 30 s, underwater 4.41 s. Floating repeats a
1.5 s sample every 2.5 s. The purported 12 s wind excerpts are shorter because
the source ends first. Music consists of two roughly 60 s excerpts from one
252.67 s source. Source ambience is longer than what we shipped: outdoor
83.78 s and cave 94.31 s. This truncation was an implementation mistake.
The current 2 s context crossfade is not a crossfade at each loop boundary.
Length alone does not remove recognizable events or an audible seam.

## Evidence and conclusions

[Audiokinetic's Wwise Project Adventure](https://www.audiokinetic.com/download/documents/WwiseProjectAdventure_en.pdf)
combines randomized ambient beds, transitions, occasional environmental details
and silence to reduce repetition and listening fatigue. Adopt those principles
with the existing backend; no middleware dependency is warranted. A quiet,
non-melodic bed should establish the environment; conspicuous bird calls or
water gestures should be sparse details, not a six-second repeating motif.

[Microsoft XAG 105](https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/105)
separates active effects from background audio and music so players can choose
what they hear clearly. The [Accessible Games Initiative](https://accessiblegames.com/resources-for-tag-implementation/)
also recommends independent important effects channels where helpful, explicitly
including footsteps. This supports a simple top-level mix plus expandable
controls for the repeated or gameplay-relevant sounds in this engine.

[EBU R128](https://tech.ebu.ch/docs/r/r128.pdf) supplies loudness and true-peak
measurement concepts, not a universal target for this interactive game. Measure
integrated/short-term loudness and peaks; do not peak-normalize everything to
full scale. [ITU-T H.872](https://www.itu.int/epublications/publication/itu-t-h-872-2024-10-safe-listening-for-video-gameplay-and-esports)
is a relevant game-listening reference. No slider position or LUFS reading can
guarantee a safe headphone level: device gain and listening duration matter.
We should make comfort claims only after listening tests.

## Recommended sound design

1. Restore the complete 84 s outdoor and 94 s cave sources as an immediate
   improvement, after editing seams. Target 90–180 s for replacement beds,
   with two or three genuinely different recordings where available. Do not
   manufacture a long file by repeating the same short clip. Dry and cold
   need new long wind recordings, rather than the same six-second source
   processed twice. Underwater needs a long subdued water bed with no regular
   bubbling phrase. Asset durations are design goals, not an external standard.
2. Prepare seamless loop boundaries and use overlapping bed transitions.
   Equal-power fades suit uncorrelated recordings; correlated layers require
   listening/peak checks to avoid a bump. Keep 2 s context transitions initially;
   try 4–8 s loop overlaps for long beds. Choose varied bed recordings without
   immediate repeats. Deterministic seeded scheduling keeps tests reproducible.
3. Replace the two cuts of one song with complete independent 3–5 minute
   tracks. The full existing 4:13 source is a usable candidate, not two tracks.
   Keep random 60–180 s music silence initially; audition a quieter 120–300 s
   alternative. Ambient may continue under music; reduce its level smoothly
   by about 3 dB during music as a starting mix experiment, not a mandate.
4. Remove the fixed floating rhythm. Use several quiet water gestures with
   irregular 6–12 s gaps while genuinely surface-floating; swimming follows
   movement. Avoid background water masking a stroke. Keep critical hurt/death
   audible, and never randomize whether gameplay events happen on the Client.
5. Audition a reference scene at fixed device gain. Begin with ambience about
   10–16 dB below a representative close action and music about 6–10 dB below;
   adjust by perceived loudness, not peak alone. These are proposed starting
   relationships, not standards. Meter the combined busy scene and leave at
   least 3 dB true-peak headroom in captured output; revise overlap/voice limits
   if peaks exceed it. Avoid compensating by compressing everything constantly.
6. Test 10–15 minutes in each Biome, cave and water, then a busy mixed scene,
   on headphones and speakers at low and ordinary volume. Include boundary
   crossings, music start/end, save/reopen and applying volumes to live sinks.
   Log audible seams, repeated landmarks, masking and fatigue.

## Asset and memory constraints

The loader currently permits only 512 KiB for each Ambient/Effects recording,
8 MiB for Music and 32 MiB for all referenced recordings in a World. At nominal
96 kbps, 180 s is about 2.16 MB before container overhead, so longer ambience
requires a separate Ambient limit (propose 4 MiB), retaining 512 KiB for Effects
and the 32 MiB World cap. Budget repeated references, not only distinct disk
files. Test transient decoding/crossfade allocations before adding many variants.
Prefer 44.1/48 kHz stereo Vorbis for beds/music, mono Effects. Encoding tools are
local authoring tools, not new Engine dependencies. The original approximately
2 MiB Terra target must grow; provisionally aim for 12–20 MiB referenced audio.

## Source screening

Existing credited [Birds and Wind](https://opengameart.org/node/93987) and
[Loopable Dungeon Ambience](https://opengameart.org/content/loopable-dungeon-ambience)
are CC0 and the downloaded full originals have measured durations above.
[Peaceful Forest](https://opengameart.org/content/peaceful-forest) is CC0 music,
not a forest field recording; audition it as a music candidate only.
[Cathedral in the forest](https://opengameart.org/content/cathedral-in-the-forest-ambient-loop)
is CC0 pads/bells for dark/space scenes; reject it for natural forest ambience
regardless of its title. Do not assume a collection's licence applies to every
entry. New wind/underwater recordings still require file-level CC0 verification,
duration measurement, credits and listening selection; no unsuitable candidate
has been silently substituted. Prefer genuinely long takes to stretched loops.

## Settings proposal

Keep Master, Effects, Ambient and Music visible. A Sound effects disclosure
contains Footsteps, Landings, Breaking & striking, Placing, Eating, Swimming,
Floating water, Pickups, Tools, and Hurt & death. Each is 0–100, relative to
Effects; Master and Effects still govern all of them. Zero mutes a subgroup.
Default the detail trims to 100 so old profiles retain their balance; perform
comfort rebalance in authored sound gains and scheduling, not hidden profile
migration. Existing top-level defaults remain 100/100/70/50 pending listening.

An additional mono-output option is worthwhile for one-ear listening, but is
outside this immediate mockup. Do not show dialogue/voice-chat/UI-volume sliders
until those sound paths exist. Avoid an unexplained generic dynamic-range preset
until its actual processing and tests are defined.

Apply, Discard and Restore defaults retain existing behavior. Audio reset only
resets Audio. The mockup has no automatic playback or fake sound preview buttons.
The original user instruction requires agreeing UI before Engine implementation;
this revision is a concrete proposal for that approval.

## Measurement baseline

Local ffmpeg loudnorm analysis of the decoded source assets before Sound,
category and Master multipliers. No assets were changed. This is not final
output metering or a listening evaluation.

| Asset | Integrated LUFS | True peak dBTP | LRA LU |
| --- | --- | --- | --- |
| open | -30.56 | -13.03 | 13.90 |
| dry | -32.14 | -21.82 | 0.10 |
| cold | -29.71 | -18.82 | 0.30 |
| water | -28.15 | -10.05 | 0.80 |
| music-1 | -40.28 | -22.64 | 12.40 |
