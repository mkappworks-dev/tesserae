const uxAudit = [
  ['Game identity',9.2,9.4,'Check that the final art represents actual Tesserae gameplay.'],
  ['Palette & restraint',8.0,9.4,'Test the sage accent on different displays and colour-vision simulations.'],
  ['Background separation',8.0,9.4,'Measure every backdrop and confirm an opaque fallback.'],
  ['Hierarchy & information density',8.8,9.3,'Observe new players finding the main action without prompting.'],
  ['Text size & typography',6.5,8.6,'Validate at 1080p, 4K and couch distance; metadata remains relatively small.'],
  ['Navigation & wayfinding',9.1,9.4,'Test the complete create, edit and return paths with new players.'],
  ['Focus & keyboard operation',7.0,9.0,'Audit every control, scrolling region, modal and focus return.'],
  ['Control targets & spacing',7.0,9.1,'Confirm custom targets and scaled layouts on touch hardware.'],
  ['Selection & status feedback',8.2,9.2,'Confirm selected, focused, enabled and disabled are never confused.'],
  ['Errors & destructive actions',7.0,9.0,'Test mistaken input, deletion, recovery and interrupted creation.'],
  ['Settings & progressive disclosure',9.0,9.3,'Test discovery of advanced options and unapplied changes.'],
  ['Responsive layout & scaling',8.5,9.0,'Test long translations and 200% text across all screen sizes.'],
  ['Controller interaction',3.0,6.5,'D-pad, A/B and shoulder logic is implemented; physical-pad and text-entry tests remain.'],
  ['Accessibility coverage',5.0,7.0,'Narration, assistive technology and disabled-player testing remain.'],
  ['Performance & engine parity',8.0,8.5,'Profile menu opening, blur cost and engine implementation on target hardware.'],
  ['Draft preservation & recovery',6.5,9.0,'Session reload is supported; test interrupted and incompatible data cases.']
];
const before=uxAudit.reduce((sum,x)=>sum+x[1],0)/uxAudit.length;
const after=uxAudit.reduce((sum,x)=>sum+x[2],0)/uxAudit.length;
document.querySelector('#scorecard').innerHTML=`<div class="audit-summary"><div><small>Before · full audit</small><strong>${before.toFixed(1)} / 10</strong></div><div><small>Refined · provisional</small><strong>${after.toFixed(1)} / 10</strong></div><div><small>Release target</small><strong>9.5+ in every area</strong></div></div><div class="audit-table-wrap"><table class="audit-table"><thead><tr><th>Aspect</th><th>Before</th><th>Refined</th><th>What earns 9.5+</th></tr></thead><tbody>${uxAudit.map(([name,baseline,current,gap])=>`<tr><td>${name}</td><td>${baseline.toFixed(1)}</td><td>${current.toFixed(1)}<div class="audit-meter"><span style="width:${current*10}%"></span></div></td><td>${gap}</td></tr>`).join('')}</tbody></table></div><p>Equal-weight mean across 16 areas. The previous 9.2 score evaluated the visual direction more narrowly; it was not a full accessibility or usability score. The scores here are explicit design judgments, not measurements of player success.</p>`;

const pageAudit=[
  ['Title screen',8.7,9.5,'Sage logo; quieter settings button; Continue card no longer wraps.'],
  ['Worlds',8.4,9.5,'Breadcrumb eyebrow; Play and Edit side by side; shorter cards.'],
  ['Create · World',8.8,9.5,'Shared game-style frames.'],
  ['Create · Mods',8.0,9.5,'Themed checkboxes; quieter category labels.'],
  ['Create · Biomes',8.2,9.5,'Sliders with readouts; colour-coded, clickable climate dots with the current name.'],
  ['Create · Review',8.9,9.5,'Shared game-style frames.'],
  ['Edit World',8.6,9.5,'Locked fields read as locked; breadcrumb matches Worlds.'],
  ['Mods library',8.3,9.5,'One quiet category label per Mod; count in the footer.'],
  ['In-game menu',8.4,9.5,'Menu header hidden in game; readable location label.'],
  ['Settings',8.6,9.5,'Themed, larger checkboxes.'],
  ['Play HUD',9.0,9.6,'Game hotbar with held-item name.'],
  ['Inventory · Catalogue',9.0,9.5,'Centred overlay, side Catalogue with drag and count transfer.'],
  ['Quick layout',8.9,9.5,'One primary action; header hidden in game.'],
  ['Survival HUD · proposed',0,9.0,'New: mosaic tiles over the hotbar, air reserved above hunger; pickup feed and engine notice card. Contrast and colour-vision checks remain.'],
  ['Death screen · proposed',0,9.0,'New: centred cause, item location and despawn time, Respawn on Enter. Couch-distance check remains.'],
  ['Game mode picker · proposed',0,9.0,'New: Create World and Edit World, default and your own mode.']
];
document.querySelector('#scorecard').insertAdjacentHTML('beforeend',`<h3 class="audit-pages">Pages</h3><div class="audit-table-wrap"><table class="audit-table"><thead><tr><th>Page</th><th>Before</th><th>Now</th><th>What changed</th></tr></thead><tbody>${pageAudit.map(([name,b,a,why])=>`<tr><td>${name}</td><td>${b?b.toFixed(1):'New'}</td><td>${a.toFixed(1)}<div class="audit-meter"><span style="width:${a*10}%"></span></div></td><td>${why}</td></tr>`).join('')}</tbody></table></div><p>Page scores judge layout, hierarchy, consistency and feedback on screen. They do not replace the controller, accessibility and player tests listed above.</p>`);
