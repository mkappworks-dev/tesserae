# Cinematic menu research and design audit

Reviewed 4 November 2026. This is a desktop-first browser prototype, not an
accessibility certification or a report of player testing.

## Recommendation

Keep Cinematic as the single direction. Use brighter slate-charcoal surfaces,
warm-white text, clear focus outlines and a restrained sage accent. Preserve
colour in the World itself and in meaningful biome previews. Keep the background
scene static; it should establish the game's identity without competing with
decisions.

The previous 9.2 assessed a visual direction with a narrower rubric. A broader,
equal-weight audit across 16 areas puts the original prototype at 7.4 and the
refined version provisionally at 8.8. The individual scores are judgment calls.
They are useful for identifying gaps, not evidence of a measurable quality
level. The live prototype includes the full scorecard and the remaining work for
each area. A 9.5+ score needs evidence across interaction, accessibility and
engine integration as well as visual polish.

## Background, blur and saturation

A recognisable scene is a good fit for this voxel game. It is not a universal
requirement for good menus. The prototype currently draws an illustrative voxel
landscape in canvas; a captured scene from the engine would better represent the
actual game. Use a static scene for the title and creation flows. An in-game
menu can use a captured view of the current World. Displaying a captured
background does not imply that the Server simulation pauses.

Use different treatment by context:

| Context                       | Treatment                                                | Reason                                             |
| ----------------------------- | -------------------------------------------------------- | -------------------------------------------------- |
| Title                         | Relatively sharp scene, darker on the text side          | Identity and atmosphere                            |
| World, Mods, Biomes, Settings | Opaque charcoal panels; scenery outside softened by 3 px | Stable control contrast and less peripheral detail |
| In-game menu                  | Mild 2 px background blur with an opaque control panel   | Separate menu interaction from gameplay            |
| Simple background option      | Solid charcoal, no scene or blur                         | Predictability and visual comfort                  |

Those pixel values are prototype choices, not research-prescribed constants. Do
not blur text, controls or their focus indicators. Strong opacity makes blur
behind the actual form panel unnecessary. Restrict the translucent treatment to
the title's action group, with a solid fallback.

Blur suppresses detail but does not guarantee adequate contrast as background
colours change. NN/g recommends checking contrast, simplifying busy backgrounds
and providing transparency controls. Microsoft's contrast guidance likewise
evaluates foregrounds against their backgrounds across contexts.
[NN/g: Glassmorphism](https://www.nngroup.com/articles/glassmorphism/),
[Xbox XAG 102: Contrast](https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/102).

Reduce the green further: large surfaces are neutral, the scene is desaturated,
and the primary accent is now muted sage `#a8b5a2`. Selected navigation also has
an arrow; selected biomes show an Editing label and an inset edge. Colour should
reinforce meaning rather than carry it alone.
[W3C: Use of Color](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html).

Avoid an animated fly-through behind forms. Microsoft specifically identifies
moving backgrounds behind stationary reading content as a potential source of
discomfort. Reduced motion and a solid background are available in the
prototype.
[Xbox XAG 117: Visual distractions and motion](https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/117).

## What matters more than decoration

**Readable text.** The previous 9–12 px metadata was too small for a
game-oriented interface. Primary controls are now 16 px and supporting copy
generally 13–14 px, with a 100–200% text-size setting. This improves the desktop
prototype but does not meet every Xbox text-size recommendation. Microsoft
recommends minimums of 18 px for PC and 26 px for console at 1080p, with larger
text and scaling for other contexts. Before game integration, verify the actual
rendered pixel size, viewing distance, languages and device DPI. Large text
reflows the header into the scrolling body so it cannot consume the entire form.
[Xbox XAG 101: Text display](https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/101).

**Predictable navigation.** The World creation order remains World → Mods →
Biomes → Review. Back and primary actions retain their locations. Settings uses
one shared category screen from both entry points. Direct URLs, browser history
and same-tab draft recovery make it easier to revisit decisions. The gamepad
prototype supports D-pad focus, A to activate, B to go back and shoulder buttons
for categories. Physical controller use and text entry still require validation.
Microsoft's guidance calls for consistent layouts, logical focus order and
complete digital-input navigation.
[Xbox XAG 112: UI navigation](https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/112).

**Obvious focus.** Focus uses a warm high-contrast outline distinct from the
sage selection state. Modals make the background inert, trap keyboard focus and
return focus to the opener. Controls scroll into view when focused. Focus must
remain perceivable and cannot disappear behind fixed chrome.
[Xbox XAG 113: Focus handling](https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/113),
[W3C: Focus Not Obscured](https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html).

**Comfortable targets.** Custom action targets generally use a 44 px minimum
height; biome checkboxes have a 44 × 44 px hit region. Some native controls
still need a full touch audit. WCAG's enhanced target criterion uses 44 × 44 CSS
pixels; its minimum criterion uses 24 × 24 with defined exceptions. The larger
size is our design preference, not a claim that 44 px is universally mandatory.
[W3C: Target Size Enhanced](https://www.w3.org/WAI/WCAG22/Understanding/target-size-enhanced.html),
[W3C: Target Size Minimum](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html).

**Recoverable decisions.** Invalid values now produce a persistent message with
focus and an accessible description on the relevant control. Deletion and reset
require confirmation. Review makes custom values inspectable. Existing Worlds
keep generation settings locked and offer creating a new World from their setup.
This follows guidance to provide context for actions and protect players from
destructive mistakes.
[Xbox XAG 114: UI context](https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/114),
[Xbox XAG 115: Errors and destructive actions](https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/115).

**Accessible entry.** Settings is reachable on the title screen before entering
a World. Continue remains a short path for returning players. A complete release
also needs narration and accessible first-launch configuration. The Game
Accessibility Guidelines cover simple controls, legibility, configuration,
remembering settings and testing with disabled players.
[Game Accessibility Guidelines: Full list](https://gameaccessibilityguidelines.com/full-list/).

## Verification and limits

Browser checks cover creation, validation, focus return, modal isolation,
settings drafts, simulated controller actions and session snapshot content. The
layout audit checks all major screens and Settings categories at standard and
doubled text size. It checks horizontal overflow, footer placement and a usable
scrolling body. It is not a complete screen-reader or gamepad audit.

The earlier browser measurements were superseded by the brighter daylight
palette. Calculated sRGB contrast ratios for its opaque colour tokens are:

- Primary text against panel: 10.98:1.
- Supporting text against panel: 7.75:1.
- Primary button text against sage fill: 10.53:1.
- Input boundary against input fill: 5.62:1.
- Primary action target height: 44 CSS px.

These samples do not certify every state, thumbnail, hover colour, background or
high-contrast setting. A control-boundary issue found during measurement was
corrected by increasing input-border contrast.

Remaining limitations include metadata size, physical controller and text-entry
behavior, narration, assistive technologies, colour-vision checks, translation,
long lists, television readability and engine profiling. Session storage is
local to the browser tab; it is not a game save or a durable project store.

## How to earn 9.5+

Treat 9.5 as a release-quality goal rather than a number to assign after a CSS
pass. First run formative sessions with representative new players, controller
users and players with relevant accessibility needs. Repeat after fixing the
observed problems; a small formative group does not establish a statistical
success rate.

Use these tasks: continue a recent World, create a seeded World with one Mod and
a custom biome, verify the result on Review, rename an existing World, change
and discard a display preference, recover from invalid biome input, and leave an
unfinished setup safely. Observe misclicks, time to find actions, errors, help
requests and whether players understand saved versus draft settings.

Require no blocked task, no inaccessible control, no hidden focus and no data
loss. Audit text at the intended viewing distance, 200% scaling and long
translations. Check all relevant colour pairs, not only the sampled defaults.
Use a real controller and the platform's supported text-entry mechanisms.
Validate screen narration. Profile the final engine's menu rendering on target
hardware; reuse a captured background instead of continuously rendering an
expensive scene solely for a static menu. Measure blur before deciding whether
to keep it in the engine.

The full scorecard's per-area acceptance work is deliberately visible in the
prototype. A strong visual direction can reach the mid-nines before the complete
interaction and accessibility experience does.

The daylight revision lifts panel and card luminance, reduces the dark scene
overlay, restores natural background colour, and softens form scenery by 1.5 px.
The new palette retains the same layout and restrained sage accents.
