# 10: Ground tint

Status: wontfix
Blocked by: 05, 09

Spec: [LOD features](../spec.md).

## What to build

Distant ground takes the colour of what grows on it, so near and far match at
the join. A coarse column's top colour is tinted by the plants level 0 would
grow there, from the same density fields Canopy cover reads. Lands only if
both gates pass; otherwise `wontfix`, with the numbers.

## Acceptance criteria

- [ ] Captures at each ring join, before and after, shown to you; lands only on
      your approval.
- [ ] Coarse-Chunk time stays within 09's budget, in Comments.

## Comments

2026-11-22: `wontfix`, on `t3/lod-ground-tint` from the integration tip
`b6dbd0f5`. Neither gate passes: no ring join's colour moves by more than a
unit on the captures, and the whole field fills 0.2 s past 10 s. The code is
kept, unmerged, on the local branch `t3/lod-ground-tint-attempt` (`2f447a49`).
By the user's change to this ticket, no human review: the captures were judged
here.

What was tried.

- `Worldgen::growth` reads, at a coarse column's centre and from the same
  density fields as Canopy cover (whose answers it leaves bit for bit), the
  share of a distant view each tree species' crowns and each Plant hide.
  `column_tints` draws a coarse top toward the largest share its Material
  leaves out: bare ground toward its Plants and, above the Impostor levels,
  trees too sparse for Canopy cover; Canopy cover toward the other species.
  The tint rides in a Quad's padding (3 bytes, a Quad stays 16) and a second
  word per table entry, and `chunk_material.wesl` mixes the palette colour in
  sixteenths. Level 0 and Generator v3 are untouched; nothing is stored.
- A first rule (v1) also drew Canopy cover and Impostor crowns toward the
  ground between the crowns, by their share seen from above.

Colour from above (`coarse_colour_from_above_against_level_zero_over_terra_windows`,
on the attempt branch): each level's top colour against level 0's, from Terra's
palette and texture averages, in 16 m blocks, sRGB mean absolute difference.
v1, as the rule then stood, over Terra's spawn and canopy windows:

| window | level | to level 0, untinted | tinted | to the finer level, untinted | tinted |
| ------ | ----- | -------------------- | ------ | ---------------------------- | ------ |
| spawn  | 1     | 4.6                  | 3.5    |                              |        |
| spawn  | 2     | 11.9                 | 8.0    | 12.4                         | 6.6    |
| spawn  | 3     | 15.8                 | 11.1   | 6.6                          | 5.5    |
| spawn  | 4     | 17.6                 | 11.6   | 12.6                         | 11.1   |
| spawn  | 5     | 23.5                 | 18.5   | 25.3                         | 21.8   |
| canopy | 1     | 6.1                  | 4.9    |                              |        |
| canopy | 2     | 12.1                 | 9.0    | 10.3                         | 6.3    |
| canopy | 3     | 13.5                 | 10.5   | 4.1                          | 3.8    |
| canopy | 4     | 15.1                 | 11.3   | 8.6                          | 8.0    |
| canopy | 5     | 21.9                 | 17.4   | 18.7                         | 19.5   |

On screen it went the other way. Seen across the ground from a distant Eye the
crowns hide the ground between them, so v1 browned the level-2 forest and
whitened pines on snow: at the six Eyes the level 1 to 2 join's colour
difference rose from 21.9 to 25.9. v2, the rule above, counts each crown at
twice its area seen across the ground and leaves crowns untinted; it changes
under 1.5% of any capture's pixels by more than 10 of 255.

Captures (not committed): `/tmp/lod22-10-UOIC/captures/`. Release `dev`
builds of `b6dbd0f5` (`before/`) and v2 (`after/`; v1 in `after-v1/`), Terra
160 m from `scripts/build-guests`, Medium (`TESSERAE_QUALITY=medium`),
offscreen 1920 x 1080, settle 30 s: 01's four Eyes, 05's two join Eyes, and two
level 0 to 1 joins, `join-0-1b` (1760.5, 70.5, -435.5, pitch -14) over a forest
and `join-0-1-meadow` (1632.5, 75.5, -560.5, pitch -12) over the spawn
Settlement's meadow. `debug-before/` is the same with `TESSERAE_LOD_TINT=1`,
which tells the levels apart. Joins measured on every third pixel column where
one level meets the next, 20 px either side, mean sRGB difference across the
join over the four 01 Eyes and two join Eyes:

| join | columns | before | v1    | v2    |
| ---- | ------- | ------ | ----- | ----- |
| 1/2  | 563     | 21.88  | 25.91 | 21.77 |
| 2/3  | 133     | 31.68  | 31.98 | 30.99 |
| 3/4  | 16      | 37.67  | 37.46 | 37.40 |
| 4/5  | 1,261   | 3.66   | 3.72  | 4.06  |

- No holes and no new seam in any capture; no join's colour closer by more than
  0.7.
- Level 0 to 1, over the meadow and the forest: no visible change (0.02 and
  0.08 mean difference over the frame). Terra's ground cover is the grass's
  colour within a few units (tuft 88,160,56 on grass 84,143,61; fern, tall
  grass), at 6 to 15% of a column, one sixteenth of tint at most.
- What does change, at levels 2 and 3: snow under sparse pines greys toward
  their leaves, meadows a little toward their flowers.

09's colour seam is not a ring join. On `debug-before/at-0.5km.png` and
`at-1km.png` the olive and the brighter forest lie in one level on either side
of the tone change (level 1 at 0.5 km; level 3, and a sliver of level 2, at 1
km): they are two forests of different leaves, as level 0 grows them.
Impostors and Canopy cover over one forest were the same Material and light
(level-2 and level-1 leaf faces all sky 15). Nothing to fix there.

Budget (`profiling` release builds, `scripts/profile-frames --quality medium
--repeats 3 --fixtures stationary sprint-flight`, offscreen 1920 x 1080, Terra
160 m; order before, v2, v2, before, each started at a 1-minute load under 3;
median of the six logging-on runs [min-max]):

| fixture               | level 0 s        | whole field s      | coarse delivered MB |
| --------------------- | ---------------- | ------------------ | ------------------- |
| before, stationary    | 1.69 [1.55-1.83] | 9.88 [8.89-10.08]  | 7.11                |
| v2, stationary        | 1.77 [1.52-2.50] | 10.11 [8.53-10.16] | 7.56                |
| before, sprint-flight | 1.67 [1.61-1.83] | 9.87 [9.58-10.42]  | 7.63                |
| v2, sprint-flight     | 1.73 [1.69-1.78] | 10.03 [9.80-10.13] | 8.25                |

Mean coarse job time by level, ms (stationary / sprint-flight), from the same
runs: level 1 5.63 / 5.28 before, 5.72 / 5.21 with the tint; level 2 1.63 /
1.70, 1.66 / 1.72; level 3 0.56 / 0.88, 0.55 / 0.87; level 4 0.69 / 0.72, 0.70 /
0.73; level 5 2.50 / 2.53, 2.59 / 2.52. The growth read adds next to nothing to
a job, but the split top faces deliver 6 to 8% more coarse vertex bytes and the
field fills 0.16 to 0.23 s later, past 09's 10 s line in both fixtures, where
it stood at 9.9 s. GPU frame was not measured.

Left open: none from this ticket. Reopen only for a Game whose ground cover
differs in colour from its ground, and then with a tint that costs no split
faces.

Ran on v1: `cargo test -p tesserae-core` (all pass), `cargo test -p tesserae
--features tesserae/dev` (1,736 passed, 9 ignored), `cargo clippy -p tesserae
-p tesserae-core --all-targets --all-features`, `cargo fmt --all --check`. v2
was built and captured, its tests not run again. This branch carries only the
ticket, the spec and DESIGN.md: `npm run fmt:check`.
