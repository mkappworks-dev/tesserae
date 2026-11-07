/* PROTOTYPE, throwaway. Step 15 Survival states and step 21 vitals, death and game mode.
   Every number below stands in for a Game declaration; the engine computes them on the Server in metres. */
const rules={maxHealth:20,maxHunger:20,maxAir:10,safeFall:3,fallPerMetre:1,drownPerSecond:2,lavaPerSecond:4,regenAbove:18,regenPerSecond:.25,sprintDrain:1,idleDrain:1,despawnSeconds:300};
carriedItems.find(x=>x.id==='melon').food=6;


function ensureSurvival(){
  state.vitals ||= {health:rules.maxHealth,hunger:rules.maxHunger,air:rules.maxAir,underwater:false};
  state.journal ||= [];
  state.mode ||= 'Creative';
  for(const w of worlds){w.mode ||= w.id===1?'Creative':'Survival';w.playerMode ||= w.mode;}
  if(state.survivalSeedRevision!==2){
    for(const x of allSlots())if(x.s?.id==='berries')x.list[x.i]=null;
    if(!state.bagHotbar[8])state.bagHotbar[8]={id:'melon',count:6,shape:'Cube'};
    state.survivalSeedRevision=2;
  }
}
function survival(){return !creativeAccess();}
function allSlots(){return [...state.bagHotbar.map((s,i)=>({list:state.bagHotbar,i,s})),...state.bag.map((s,i)=>({list:state.bag,i,s}))];}
function holding(id){return allSlots().filter(x=>x.s?.id===id).reduce((n,x)=>n+x.s.count,0);}
function itemName(id){return carriedItems.find(x=>x.id===id)?.name||id;}

/* Pickup and refunds fill the hotbar's matching stacks, then the backpack, then empty slots. Returns what did not fit. */
function receive(id,count){
  const max=carriedItems.find(x=>x.id===id).maxStack;let left=count;
  for(const {s} of allSlots())if(left&&s?.id===id&&(s.shape||'Cube')==='Cube'){const n=Math.min(left,max-s.count);s.count+=n;left-=n;}
  for(const {list,i,s} of allSlots())if(left&&!s){const n=Math.min(left,max);list[i]={id,count:n,shape:'Cube'};left-=n;}
  return left;
}
function spend(id,count){
  if(holding(id)<count)return false;state.pickupRefused=null;let left=count;
  for(const x of allSlots())if(left&&x.s?.id===id){const n=Math.min(left,x.s.count);x.s.count-=n;left-=n;if(!x.s.count)x.list[x.i]=null;}
  return true;
}

function hurt(amount,cause){
  if(!survival()){log('Creative: no damage.');return;}
  const v=state.vitals;v.health=Math.max(0,v.health-amount);state.hitAt=Date.now();
  log(`${cause.short}: ${amount} damage.`);
  if(!v.health){die(cause);return;}
  render();
}
function die(cause){
  const dropped=state.keepInventory?[]:allSlots().filter(x=>x.s).map(x=>({...x.s}));
  if(!state.keepInventory){state.bag=state.bag.map(()=>null);state.bagHotbar=state.bagHotbar.map(()=>null);}
  state.death={cause,dropped,at:'412, 63, −88',distance:140,diedAt:Date.now()};
  state.journal=[];state.pickupRefused=null;navigate('death');
}
function respawn(){
  state.vitals={health:rules.maxHealth,hunger:rules.maxHunger,air:rules.maxAir,underwater:false};
  const d=state.death;state.death=null;navigate('play');
  const left=d?despawnLeft(d):0;
  notice(d?.dropped.length?left?`Back at the World spawn. Your items are ${d.distance} m away, for ${clock(left)}.`:'Back at the World spawn. Your items have despawned.':'Back at the World spawn.','info',5);
}
/* The Server owns the despawn deadline; the death screen counts down to it and keeps counting while it is open. */
function despawnLeft(d){return Math.max(0,Math.ceil(rules.despawnSeconds-(Date.now()-(d.diedAt??Date.now()))/1000));}
function clock(s){return `${Math.floor(s/60)}:${String(s%60).padStart(2,'0')}`;}
setInterval(()=>{
  const el=$('#despawn-left');if(!el||state.screen!=='death')return;
  const left=despawnLeft(state.death||deathScreen.sample);
  el.textContent=left?`They stay for ${clock(left)}.`:'They have despawned.';
},1000);
function tickTime(seconds,sprinting){
  const v=state.vitals;
  if(!survival()){log('Creative: hunger does not drain.');return;}
  v.hunger=Math.max(0,v.hunger-(sprinting?rules.sprintDrain*2:rules.idleDrain));
  if(v.hunger>=rules.regenAbove)v.health=Math.min(rules.maxHealth,v.health+Math.floor(seconds*rules.regenPerSecond));
  log(`${seconds} s ${sprinting?'sprinting':'idle'}.`);
  if(!v.hunger){hurt(1,{title:'You starved',detail:'Hunger reached zero.',short:'Starved'});return;}
  render();
}

function placeHeld(){
  const stack=state.bagHotbar[state.quickSlot],item=stackItem(stack);
  if(!stack){notice(`Nothing to place in hotbar slot ${state.quickSlot+1}.`,'warn');return;}
  stack.count--;if(!stack.count)state.bagHotbar[state.quickSlot]=null;state.pickupRefused=null;
  state.journal.push({kind:'place',id:item.id});
  handFx('swing');
  log(`Placed ${item.material}.`);render();
}
function breakVoxel(){
  state.journal.push({kind:'remove',id:'rock'});
  const left=receive('rock',1);
  state.pickupRefused=left?'1 Rock':null;
  if(!left)feed('rock',1);
  handFx('break');
  log('Removed Stone.');render();
}
function undo(){
  const last=state.journal.at(-1);
  if(!last){notice('Nothing to undo.','warn');return;}
  if(last.kind==='place'){
    state.journal.pop();
    const left=receive(last.id,1);
    if(left)notice(`Your Inventory is full: 1 ${itemName(last.id)} dropped at your feet.`,'info');
    log(`Undid the placement${left?'':'; 1 '+itemName(last.id)+' refunded'}.`);render();return;
  }
  if(survival()&&!spend(last.id,1)){notice(`Can’t undo: putting the ${carriedItems.find(x=>x.id===last.id)?.material||'Voxel'} back needs 1 ${itemName(last.id)}.`,'warn');return;}
  state.journal.pop();log(`Undid the removal${survival()?'; 1 '+itemName(last.id)+' spent':''}.`);render();
}
function pickUp(id,count){
  const left=receive(id,count),got=count-left;
  state.pickupRefused=left?`${left} ${itemName(id)}`:null;
  if(got)feed(id,got);
  log(`Pickup of ${count} ${itemName(id)}: ${got} taken, ${left} left.`);render();
}
function fillBackpack(){for(let i=0;i<state.bag.length;i++)state.bag[i] ||= {id:'lamp',count:64,shape:'Cube'};for(let i=0;i<9;i++)state.bagHotbar[i] ||= {id:'lamp',count:64,shape:'Cube'};log('Every slot holds a stack.');render();}
function eat(){
  const stack=state.bagHotbar[state.quickSlot],item=stackItem(stack),v=state.vitals;
  if(!item?.food){notice('Hold food in the selected hotbar slot to eat.','warn');return;}
  /* Creative never drains, and the bar rounds up: both refuse before anything is spent. */
  if(!survival()||Math.ceil(v.hunger)>=Math.ceil(rules.maxHunger)){notice('You aren’t hungry.','warn');return;}
  if(state.eating)return;
  /* Eating takes a second, as a removal takes its time: the first bite spends the Item and each of
     the three bites refills a third. Changing slot stops it, and what was not eaten stays unfilled. */
  const slot=state.quickSlot,each=item.food/3;state.eating={slot};
  handFx('bite');render();
  [450,650,850].forEach((at,i)=>setTimeout(()=>{
    if(!state.eating||state.quickSlot!==slot||state.screen!=='play')return;
    if(!i){stack.count--;if(!stack.count)state.bagHotbar[slot]=null;state.pickupRefused=null;}
    v.hunger=Math.min(rules.maxHunger,v.hunger+each);
    if(i===2){state.eating=null;log(`Ate ${item.name}.`);}
    render();
  },at));
  setTimeout(()=>{state.eating=null;},1000);
}

/* The first-person hand. Only a Server-accepted action starts a motion; reduced motion keeps it still. */
const HAND_FX_MS={swing:200,break:400,bite:1000,switch:240};
function handFx(kind){
  if($('#game')?.classList.contains('reduced-motion'))return;
  state.handFx={kind,at:Date.now()};
  setTimeout(()=>{if(state.handFx&&Date.now()-state.handFx.at>=HAND_FX_MS[kind]&&state.screen==='play'){state.handFx=null;render();}},HAND_FX_MS[kind]+30);
}
function handMarkup(stack){
  const item=stackItem(stack),fx=state.handFx&&Date.now()-state.handFx.at<HAND_FX_MS[state.handFx.kind]?state.handFx.kind:'';
  const motion=fx==='break'?'swing':fx||(state.aimHelper==='breaking'?'mine':'');
  const rock=carriedItems.find(x=>x.id==='rock')?.color||'#8d9087';
  const burst=fx==='break'?`<div class="burst" aria-hidden="true">${[...Array(8)].map((_,i)=>`<i style="--dx:${(i-3.5)*9}px;--dy:${-18-(i%3)*10}px;background:${rock}"></i>`).join('')}</div>`:'';
  const crack=state.aimHelper==='breaking'?'<div class="crack" aria-hidden="true"></div>':'';
  return `${crack}${burst}<div class="hand ${motion}" aria-hidden="true"><i class="arm"></i>${item?voxelIcon(item):''}</div>`;
}

/* The engine's notice card (menu_ui::notify): top right, one at a time, 2.6 s by default. */
function notice(text,tone='info',seconds=2.6){
  const card=$('#notice-card');card.textContent=text;card.dataset.tone=tone;
  card.classList.remove('show');void card.offsetWidth;card.classList.add('show');
  clearTimeout(notice.timer);notice.timer=setTimeout(()=>card.classList.remove('show'),seconds*1000);
  log(text);
}
const feedRows=[];
function feed(id,count){
  const row=feedRows.find(r=>r.id===id);
  if(row){row.count+=count;clearTimeout(row.timer);feedRows.splice(feedRows.indexOf(row),1);}
  const target=row||{id,count};feedRows.unshift(target);
  target.timer=setTimeout(()=>{feedRows.splice(feedRows.indexOf(target),1);drawFeed();},3000);
  if(feedRows.length>5)feedRows.length=5;
  drawFeed();
}
function drawFeed(){$('#pickup-feed').innerHTML=feedRows.map(r=>{const item=carriedItems.find(x=>x.id===r.id);return `<div class="feed-row">${voxelIcon(item)}<b>+${r.count}</b><span>${esc(item.name)}</span></div>`;}).join('');}
function log(text){state.simLog=text;}

function tiles(value,max,kind){
  const n=max/2;let html='';
  for(let i=0;i<n;i++){const fill=value>=(i+1)*2?'full':value===i*2+1?'half':'empty';html+=`<span class="vital-tile ${kind} ${fill}"></span>`;}
  return html;
}
function survivalHud(){
  ensureSurvival();
  const v=state.vitals,low=v.health<=6,full=state.pickupRefused?`<div class="hud-notice" role="status">Inventory full · ${esc(state.pickupRefused)} left on the ground</div>`:'';
  if(!survival())return {above:full,overlay:''};
  const air=v.underwater||v.air<rules.maxAir;
  const overlay=`${low?'<div class="low-health-vignette" aria-hidden="true"></div>':''}${Date.now()-(state.hitAt||0)<400?'<div class="hit-flash" aria-hidden="true"></div>':''}`;
  const label=`Health ${v.health} of ${rules.maxHealth}, hunger ${v.hunger} of ${rules.maxHunger}${air?`, air ${v.air} of ${rules.maxAir}`:''}`;
  return {overlay,above:`${full}<div class="vital-rows" role="group" aria-label="${label}"><div class="vital-air-row" ${air?'':'aria-hidden="true" data-idle="true"'}><div class="vital-tiles air">${tiles(v.air*2,rules.maxAir*2,'air')}</div></div><div class="vital-main-row"><div class="vital-tiles health ${low?'low':''}">${tiles(v.health,rules.maxHealth,'health')}</div><div class="vital-tiles hunger">${tiles(v.hunger,rules.maxHunger,'hunger')}</div></div></div>`};
}

function deathScreen(){
  ensureSurvival();
  const d=state.death||(deathScreen.sample ||= {cause:{title:'You fell too far',detail:'Fell 25 m. Falls over 3 m hurt.',short:'Fell 25 m'},dropped:[{id:'rock',count:40},{id:'dirt',count:24}],at:'412, 63, −88',distance:140,diedAt:Date.now()});
  const left=despawnLeft(d);
  const kept=state.keepInventory,count=d.dropped.reduce((n,s)=>n+s.count,0);
  const items=kept?'You keep your Inventory: this Game declares keep_inventory.':d.dropped.length?`${count} items in ${d.dropped.length} stacks lie where you fell, at ${d.at}.`:'You carried nothing.';
  const actions=`${button('Respawn <span>Enter</span>','respawn',{primary:true})}${button('Return to Worlds','leave',{ghost:true})}`;
  const where=`${activeWorld()?.name||'Your World'} · ${d.distance} m from the World spawn`;
  return `<div class="death-screen"><section class="death-centre" role="alertdialog" aria-labelledby="death-title"><p class="eyebrow">${esc(where).toUpperCase()}</p><h1 id="death-title">${esc(d.cause.title)}</h1><p class="death-cause">${esc(d.cause.detail)}</p><div class="notice">${esc(items)}${!kept&&d.dropped.length?` <span id="despawn-left" aria-live="off">${left?`They stay for ${clock(left)}.`:'They have despawned.'}</span>`:''}</div><div class="death-actions">${actions}</div></section></div>`;
}

function modePicks(key,value,{legend,hint,compact}){
  const modes={Creative:'Take any Material from the Catalogue. Placing spends Items, as in Survival. No damage or hunger.',Survival:'Gather what you place. Health and hunger matter, and the Catalogue is closed.'};
  return `<fieldset class="mode-picks ${compact?'compact':''}"><legend class="section-label">${legend}</legend><div class="mode-options">${Object.entries(modes).map(([name,about])=>`<button type="button" class="mode-pick ${value===name?'selected':''}" data-action="pick-mode" data-value="${key}:${name}" aria-pressed="${value===name}"><b>${name}${value===name?' ✓':''}</b><small>${about}</small></button>`).join('')}</div>${hint?`<small class="mode-hint">${hint}</small>`:''}</fieldset>`;
}

function simPanel(){
  if(!['play'].includes(state.screen))return '';
  const v=state.vitals,mode=survival()?'Survival':'Creative';
  const b=(label,act,val)=>`<button data-action="${act}" ${val!==undefined?`data-value="${val}"`:''}>${label}</button>`;
  return `<aside class="sim-panel" aria-label="Prototype simulation"><header><b>Simulate</b><span>${mode} · not part of the design</span><button data-action="sim-toggle" aria-expanded="${!state.simCollapsed}">${state.simCollapsed?'Show':'Hide'}</button></header>${state.simCollapsed?'':`<div class="sim-group"><small>Items</small>${b('Place held (right-click)','sim-place')}${b('Break Stone','sim-break')}${b('Undo (Z)','sim-undo')}${b('Pick up 12 Dirt','sim-pickup','dirt:12')}${b('Fill every slot','sim-fill')}</div><div class="sim-group"><small>Body</small>${b('Fall 9 m','sim-fall',9)}${b('Fall 25 m','sim-fall',25)}${b(v.underwater?'Surface':'Dive','sim-dive')}${b('2 s underwater','sim-breath')}${b('Touch lava 1 s','sim-lava')}${b('Sprint 10 s','sim-time','sprint')}${b('Wait 60 s','sim-time','idle')}${b('Eat held (G)','sim-eat')}${b('Starve','sim-starve')}${b('Die, items despawned','sim-despawned')}</div><div class="sim-group"><small>Engine states</small>${b('Hold remove','sim-aim','breaking')}${b('Place hint','sim-aim','place')}${b('Measure tool','sim-aim','measure')}${b('Hold Fire','sim-tool','Fire')}${b('Saving','sim-save','saving')}${b('Save failed','sim-save','error')}${b('Save failed on quit','sim-save','quit-error')}${b('Exit without saving','sim-save','confirm')}</div><div class="sim-group"><small>Game declares</small><small>Melon: food 6 · regeneration ${rules.regenPerSecond} health/s at hunger ${rules.regenAbove}+</small><label><input type="checkbox" data-bind="keepInventory" ${state.keepInventory?'checked':''}> keep_inventory</label></div><pre class="sim-state">health ${v.health}/${rules.maxHealth}  hunger ${v.hunger}/${rules.maxHunger}  air ${v.air}/${rules.maxAir}${v.underwater?' (underwater)':''}
journal ${state.journal.map(j=>j.kind+':'+j.id).slice(-4).join(' ')||'empty'}
last ${state.simLog||'—'}
held ${state.bagHotbar[state.quickSlot]?itemName(state.bagHotbar[state.quickSlot].id)+' × '+state.bagHotbar[state.quickSlot].count:'nothing'}  ·  ${allSlots().filter(x=>x.s).length}/36 slots used</pre>`}</aside>`;
}

function survivalAction(name,value){
  if(name==='pick-mode'){const [key,mode]=value.split(':');state[key]=mode;state.dirty=true;const y=$('.body')?.scrollTop;render();if($('.body'))$('.body').scrollTop=y;return true;}
  if(name==='respawn'){respawn();return true;}
  if(name==='sim-toggle'){state.simCollapsed=!state.simCollapsed;render();return true;}
  if(name==='sim-place'){placeHeld();return true;}
  if(name==='sim-break'){breakVoxel();return true;}
  if(name==='sim-undo'){undo();return true;}
  if(name==='sim-pickup'){const [id,n]=value.split(':');pickUp(id,Number(n));return true;}
  if(name==='sim-fill'){fillBackpack();return true;}
  if(name==='sim-fall'){const m=Number(value),dmg=Math.max(0,(m-rules.safeFall)*rules.fallPerMetre);if(!dmg){log('A safe landing.');render();return true;}hurt(dmg,{title:'You fell too far',detail:`Fell ${m} m. Falls over ${rules.safeFall} m hurt.`,short:`Fell ${m} m`});return true;}
  if(name==='sim-dive'){state.vitals.underwater=!state.vitals.underwater;if(!state.vitals.underwater)state.vitals.air=rules.maxAir;render();return true;}
  if(name==='sim-breath'){const v=state.vitals;if(!survival()){log('Creative: no air limit.');render();return true;}if(!v.underwater){log('Dive first.');render();return true;}if(v.air>0){v.air=Math.max(0,v.air-2);render();}else hurt(rules.drownPerSecond*2,{title:'You ran out of air',detail:'Underwater with no breath left.',short:'Drowned'});return true;}
  if(name==='sim-lava'){hurt(rules.lavaPerSecond,{title:'You touched lava',detail:'Lava hurts while you touch it.',short:'Burned'});return true;}
  if(name==='sim-time'){tickTime(value==='sprint'?10:60,value==='sprint');return true;}
  if(name==='sim-eat'){eat();return true;}
  if(name==='sim-starve'){hurt(rules.maxHealth,{title:'You starved',detail:'Hunger reached zero.',short:'Starved'});return true;}
  if(name==='sim-despawned'){survivalAction('sim-fall',25);if(state.death)state.death.diedAt=Date.now()-3600e3;render();return true;}
  if(name==='sim-aim'){state.aimHelper=state.aimHelper===value?null:value;render();return true;}
  if(name==='sim-tool'){state.heldTool=state.heldTool?null:value;render();return true;}
  if(name==='sim-save'){if(value==='saving')notice('Saving World…','info',3);else if(value==='error')dialog('Could not save World','<p>Your World is still open. Your latest changes have not been saved.</p><div class="notice">Disk full</div>',button('Retry save','close-dialog',{primary:true}));else if(value==='quit-error')dialog('Could not save World','<p>Your World is still open. Your latest changes have not been saved.</p><div class="notice">Disk full</div>',button('Return to World','close-dialog',{ghost:true})+button('Retry Save & Quit','close-dialog',{primary:true})+button('Exit without saving…','sim-save',{value:'confirm',danger:true,small:true}));else dialog('Exit without saving?','<p>Unsaved World changes will be lost. This cannot be undone.</p>',button('Exit without saving','close-dialog',{danger:true})+button('Cancel','close-dialog',{primary:true}));return true;}
  return false;
}

document.addEventListener('change',e=>{if(e.target.dataset.bind==='keepInventory'){state.keepInventory=e.target.checked;savePrototype();e.stopImmediatePropagation();}},true);
window.addEventListener('keydown',e=>{
  if(!['play','death'].includes(state.screen)||$('#dialog-root').children.length)return;
  if(e.target.matches?.('input,textarea,select')||e.target.isContentEditable)return;
  if(state.screen==='death'){
    if(e.key==='Enter'&&!e.target.closest?.('button')){e.preventDefault();e.stopImmediatePropagation();respawn();return;}
    if(['Escape','F10','i','c','I','C'].includes(e.key)){e.preventDefault();e.stopImmediatePropagation();return;}
  }
  if(state.screen==='play'&&e.key.toLowerCase()==='g'){e.preventDefault();e.stopImmediatePropagation();eat();return;}
  if(state.screen==='play'&&e.key.toLowerCase()==='z'&&!e.metaKey&&!e.ctrlKey){e.preventDefault();e.stopImmediatePropagation();undo();}
},true);
document.addEventListener('contextmenu',e=>{if(state.screen==='play'&&e.target.closest('#screen')&&!e.target.closest('.sim-panel,.hotbar')){e.preventDefault();placeHeld();}});

function placeToast(){
  const hud=state.screen==='play'&&$('.hud-bottom'),t=$('#toast');
  if(!hud){t.style.bottom='';return;}
  t.style.bottom=($('#game').getBoundingClientRect().bottom-hud.getBoundingClientRect().top+10)+'px';
}
window.addEventListener('resize',()=>placeToast());
