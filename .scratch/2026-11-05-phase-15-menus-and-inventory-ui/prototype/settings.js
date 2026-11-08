const settingsDefaults = {
  // The engine's defaults (profile.rs, actions.rs).
  sprint:'Walk', removal:'Hold', repeat:false, mouse:1, invert:false,
  scroll:'Standard', scrollSpeed:1, reference:false, padLook:1,
  padInvert:false, moveDeadzone:.15, lookDeadzone:.15, crouch:'Toggle',
  touchLook:1, touchInvert:false, touchLayout:'Normal', touchSize:'Normal',
  touchOpacity:.7, sync:'Display synced', aa:'SMAA', renderScale:'100%',
  fps:'No limit', sound:100, music:50, effects:100, ambient:70, footsteps:100, landings:100, breaking:100, placing:100, eating:100, swimming:100, floating:100, pickups:100, tools:100, vitals:100, motion:false, contrast:false, simpleBackground:false, textScale:"1",
  // Engine action names and key labels (settings/pages.rs:5-57, actions.rs:265-370).
  bindings:{'Move forward':'W','Move backward':'S','Move left':'A','Move right':'D',
    'Ascend / jump':'Space','Cycle sprint speed':'Left Shift','Descend / crouch':'Left Ctrl','Place / ignite':'Right mouse',
    'Remove':'Left mouse','Search the Catalogue':'C','Open Inventory':'I','Flip Shape':'E','Layout mode and menu':'L',
    'Open the Menu':'Tab','Release cursor':'Escape','Open Settings':'F10',
    'Respawn':'R','Switch flight / physics':'V','Switch first / third person':'F5',
    'Third-person zoom in':'Alt+Wheel up','Third-person zoom out':'Alt+Wheel down',
    'Increase flight speed':'=','Decrease flight speed':'-',
    'Sample':'P','Empty hand':'X','Next Shape':'F','Turn Shape':'Q',
    'Take tool':'M','Use tool':'G','Clear tool':'Backspace',
    'Layout: more Voxels':'Shift+Wheel up','Layout: fewer Voxels':'Shift+Wheel down',
    'Layout: reverse the axis':'T','Layout: next axis':'J',
    'Undo':'Ctrl/Cmd+Z','Redo':'Ctrl/Cmd+Shift+Z',
    'Quick slot 1':'1','Quick slot 2':'2','Quick slot 3':'3','Quick slot 4':'4','Quick slot 5':'5',
    'Quick slot 6':'6','Quick slot 7':'7','Quick slot 8':'8','Quick slot 9':'9',
    'Previous quick slot':'','Next quick slot':'','Open input settings':'F1','Diagnostics':'F3'},
  // The engine keeps a second binding per action; an empty one reads "Add binding".
  altBindings:{'Cycle sprint speed':'Right Shift','Sample':'Middle mouse','Redo':'Ctrl+Y',
    'Layout: more Voxels':']','Layout: fewer Voxels':'['},
  padBindings:{'Ascend / jump':'A','Descend / crouch':'B','Open the Menu':'Y','Next Shape':'X','Cycle sprint speed':'Left stick',
    'Sample':'Right stick','Remove':'RT','Place / ignite':'LT','Previous quick slot':'LB','Next quick slot':'RB',
    'Turn Shape':'D-pad left','Flip Shape':'D-pad right','Empty hand':'D-pad down','Use tool':'D-pad up',
    'Open input settings':'Start','Switch flight / physics':'Select'}
};
// Audio includes the proposed detailed effect mix.
const settingsCategories = ['Behavior','Key bindings','Controller','Display','Audio','Accessibility'];
// The engine's order and categories (settings/pages.rs binding_order, category).
const bindingGroups = {
  Movement:['Move forward','Move backward','Move left','Move right','Ascend / jump','Descend / crouch','Cycle sprint speed',
    'Respawn','Switch flight / physics','Increase flight speed','Decrease flight speed',
    'Switch first / third person','Third-person zoom in','Third-person zoom out'],
  Building:['Place / ignite','Remove','Sample','Empty hand','Turn Shape','Flip Shape','Next Shape','Take tool',
    'Use tool','Clear tool','Layout mode and menu','Layout: more Voxels','Layout: fewer Voxels',
    'Layout: reverse the axis','Layout: next axis'],
  Interface:['Open the Menu','Open Inventory','Search the Catalogue','Open input settings','Open Settings',
    'Release cursor','Undo','Redo','Diagnostics','Quick slot 1','Quick slot 2','Quick slot 3','Quick slot 4',
    'Quick slot 5','Quick slot 6','Quick slot 7','Quick slot 8','Quick slot 9','Previous quick slot',
    'Next quick slot']
};

function bindingGroup(name) {
  return Object.keys(bindingGroups).find(g=>bindingGroups[g].includes(name))||'Interface';
}

function bindingRows() {
  const d=state.settingsDraft,query=(state.bindingSearch||'').trim().toLowerCase();
  const names=query?Object.values(bindingGroups).flat():bindingGroups[state.bindingGroup||'Movement'];
  const rows=names.filter(n=>!query||`${n} ${d.bindings[n]} ${d.altBindings[n]||''}`.toLowerCase().includes(query)).map(n=>{
    const primary=d.bindings[n],alt=d.altBindings[n];
    const changed=primary!==settingsDefaults.bindings[n]||(alt||'')!==(settingsDefaults.altBindings[n]||'');
    return `<div class="setting-row binding-row"><div class="setting-copy"><b>${esc(n)}</b>${query?`<small>${bindingGroup(n)}</small>`:''}</div><div class="row">${primary?button(esc(primary),'settings-rebind',{value:n}):button('Add binding','settings-rebind',{value:n,ghost:true})}${alt?button(esc(alt),'settings-rebind-alt',{value:n}):button('Add binding','settings-rebind-alt',{value:n,ghost:true})}${changed?button('↺','settings-binding-reset',{value:n,ghost:true,small:true}):''}</div></div>`;
  });
  return rows.join('')||'<div class="empty-state">No matches. Try another name or clear your search.</div>';
}

function ensureSettings() {
  if (!state.settingsDraft) {
    state.settings={...clone(settingsDefaults),...state.settings};
    for(const key of ['bindings','altBindings','padBindings'])state.settings[key]={...clone(settingsDefaults[key]),...state.settings[key]};
    state.settingsDraft=clone(state.settings);
    state.settingsDirty=false;
  }
  state.settingsTab ||= 'Behavior';
}

function settingsControl(label,key,kind,options=[],hint='') {
  const value=state.settingsDraft[key];
  let control;
  if(kind==='toggle') control=`<input class="toggle" aria-label="${label}" data-bind="settings.${key}" type="checkbox" ${value?'checked':''}>`;
  else if(kind==='range') control=`<div class="setting-slider"><output id="setting-${key}">${value}</output><input class="input" aria-label="${label}" data-bind="settings.${key}" type="range" min="${options[0]}" max="${options[1]}" step="${options[2]}" value="${value}"></div>`;
  else control=customSelect(label,'settings.'+key,value,options);
  return `<div class="setting-row"><div class="setting-copy"><b>${label}</b>${hint?`<small>${hint}</small>`:''}</div>${control}</div>`;
}

function settingsBody() {
  const row=settingsControl;
  const group=(name)=>`<p class="section-label">${name}</p>`;
  switch(state.settingsTab) {
    case 'Behavior': return group('Movement & building')+
      row('Sprint speed','sprint','select',['Walk','Speed 1','Speed 2','Speed 3'],'Sprint cycles through these and keeps the one you stop on.')+
      row('Removal activation','removal','select',['Hold','Timed tap'])+
      row('Repeat while held','repeat','toggle',[],'Keep placing or removing while the binding is held.')+
      group('Mouse & quick slots')+
      row('Mouse sensitivity','mouse','range',[.25,3,.05],'Relative to the default mouse feel.')+
      row('Invert vertical look','invert','toggle')+
      row('Scroll direction','scroll','select',['Standard','Reversed'])+
      row('Scroll sensitivity','scrollSpeed','range',[.25,3,.05])+
      row('Show quick reference','reference','toggle');
    case 'Key bindings': return `<div class="search-line"><input class="input" aria-label="Search actions or keys" data-bind="bindingSearch" placeholder="Search actions or keys…" value="${esc(state.bindingSearch||'')}"></div>`+
      (state.bindingSearch?`<p class="muted">Searching all categories</p>`:`<div class="library-tabs">${Object.keys(bindingGroups).map(n=>`<button class="${(state.bindingGroup||'Movement')===n?'active':''}" data-action="settings-binding-group" data-value="${n}">${n}</button>`).join('')}</div>`)+
      `<div class="row between"><span class="muted">Keyboard & mouse · primary and alternative</span><span class="pill">Select to rebind</span></div><div id="binding-results">${bindingRows()}</div>`;
    case 'Controller': return group('Sticks & buttons')+
      row('Look sensitivity','padLook','range',[.25,3,.05])+
      row('Invert vertical look','padInvert','toggle')+
      row('Movement deadzone','moveDeadzone','range',[.05,.5,.01],'Ignore small movements caused by stick drift.')+
      row('Look deadzone','lookDeadzone','range',[.05,.5,.01])+
      row('Crouch activation','crouch','select',['Hold','Toggle'])+
      `<details><summary>Touch controls</summary>`+
      row('Look sensitivity','touchLook','range',[.25,3,.05])+
      row('Invert vertical look','touchInvert','toggle')+
      row('Layout','touchLayout','select',['Normal','Mirrored'])+
      row('Control size','touchSize','select',['Small','Normal','Enlarged'])+
      row('Control opacity','touchOpacity','range',[.3,1,.01])+`</details>`+
      group('Controller buttons')+
      `<p class="muted">The left stick moves and the right stick looks. In menus the D-pad or left stick moves focus, A selects, B goes back, LB and RB switch pages, and Start closes.</p>`+
      Object.values(bindingGroups).flat().filter(n=>n!=='Release cursor').map(n=>{const b=state.settingsDraft.padBindings[n];return `<div class="setting-row binding-row"><b>${esc(n)}</b>${b?button(esc(b),'settings-rebind-pad',{value:n}):button('Add binding','settings-rebind-pad',{value:n,ghost:true})}</div>`;}).join('');
    case 'Display': return group('Frame presentation')+
      row('Display sync','sync','select',['Display synced','Sync off'],'Match frame presentation to your display.')+
      `<div class="notice">${state.settingsDraft.sync==='Sync off'?'Sync is off. Frames may tear; choose a limit below to reduce unnecessary work.':'Recommended for normal play. Keeps presentation free of tearing.'}</div>`+
      row('Frame rate limit','fps','select',['60','120','144','240','1000','No limit'],'An optional ceiling, independent of display sync.')+
      `<div class="setting-row"><div class="setting-copy"><b>${state.settingsDraft.sync==='Sync off'?(state.settingsDraft.fps==='No limit'?'Uncapped rendering':'Sync off'):'Display synced'}</b><small>${state.settingsDraft.fps==='No limit'?'No additional frame limit':`${state.settingsDraft.fps} FPS ceiling`}</small></div><span class="pill">Frame pacing</span></div>`+
      group('Image quality')+
      row('Anti-aliasing','aa','select',['Off','SMAA','MSAA 4x'],'Smooth voxel edges. MSAA costs more.')+
      row('Render scale','renderScale','select',['67%','75%','85%','100%'],'Lower values reduce rendering work. Menus stay sharp.')+
      `<p class="muted">View distance, textures, shadows, and simulation settings stay unchanged.</p>`;
    case 'Audio': return group('Sound mix')+
      row('Master volume','sound','range',[0,100,1],'Everything the World plays.')+
      row('Effects','effects','range',[0,100,1],'All action sounds, including the fine controls below.')+
      row('Ambient','ambient','range',[0,100,1],'Environmental beds around you.')+
      row('Music','music','range',[0,100,1],'Occasional tracks, with silence between them.')+
      `<details class="audio-details"><summary>Fine-tune sound effects</summary><p class="muted">Relative to Effects. Set any sound to 0 to mute it.</p>`+
      row('Footsteps','footsteps','range',[0,100,1],'Walking and running on each Material.')+
      row('Landings','landings','range',[0,100,1],'The impact when you land.')+
      row('Breaking & striking','breaking','range',[0,100,1],'Removing and striking Voxels.')+
      row('Placing','placing','range',[0,100,1],'Voxels placed in the World.')+
      row('Eating','eating','range',[0,100,1],'Bites while eating food.')+
      row('Swimming','swimming','range',[0,100,1],'Strokes while moving through water.')+
      row('Floating water','floating','range',[0,100,1],'Gentle water movement while resting at the surface.')+
      row('Pickups','pickups','range',[0,100,1],'Items joining your Inventory.')+
      row('Tools','tools','range',[0,100,1],'Successful measurements and refused uses.')+
      row('Hurt & death','vitals','range',[0,100,1],'Feedback when you take damage or die.')+
      `</details><div class="notice">Apply changes, then resume to hear your mix. World audio rests while Settings is open.</div>`;
    case 'Accessibility': return `<div class="row between"><p class="section-label">Comfort & readability</p><span class="pill">Proposed category</span></div>`+
      row('Reduced motion','motion','toggle',[],'Use immediate interface transitions.')+
      row('Stronger UI contrast','contrast','toggle',[],'Increase panel borders and secondary text contrast.')+
      row('Simple menu background','simpleBackground','toggle',[],'Replace the scenery with a solid background.')+
      row('Text size','textScale','select',['1','1.25','1.5','1.75','2'],'100% is standard; 200% doubles the text in menus and the HUD.');
  }
}

function settings() {
  ensureSettings();
  const icons=['behavior','keyboard','controller','display','audio','accessibility'];
  const nav=`<nav class="rail settings-rail" aria-label="Settings categories"><div class="rail-title">SETTINGS</div>${settingsCategories.map((name,i)=>`<button class="${state.settingsTab===name?'active':''}" data-action="settings-category" data-value="${name}" ${state.settingsTab===name?'aria-current="page"':''}><span class="category-icon">${icon(icons[i])}</span><span class="nav-label">${name}</span><span class="nav-arrow">${icon('chevron',12)}</span></button>`).join('')}</nav>`;
  return frame({title:state.settingsTab,subtitle:'Your preferences, across every World.',eyebrow:'MENU / SETTINGS',rail:nav,body:settingsBody(),footer:`${button('← Back','settings-back',{ghost:true})}<div class="row wrap"><span class="footer-hint" id="settings-status">${state.settingsDirty?'Unapplied changes':'All changes applied'}</span>${button('Restore defaults','settings-reset',{ghost:true,small:true})}${button('Apply changes','settings-apply',{primary:true,disabled:!state.settingsDirty})}</div>`});
}

function settingsChanged() {
  state.settingsDirty=JSON.stringify(state.settingsDraft)!==JSON.stringify(state.settings);
  const apply=document.querySelector('[data-action=settings-apply]');
  if(apply)apply.disabled=!state.settingsDirty;
  if($('#settings-status'))$('#settings-status').textContent=state.settingsDirty?'Unapplied changes':'All changes applied';
}

function applySettings() {
  state.settings=clone(state.settingsDraft);
  state.settingsDirty=false;
  applyComfort();
  $('#game').classList.toggle('strong-contrast',state.settings.contrast);
  $('#game').classList.toggle('reduced-motion',state.settings.motion);
  document.documentElement.style.scrollBehavior=state.settings.motion?'auto':'smooth';
}

function settingsAction(name,value) {
  if(name==='settings-category'){state.settingsTab=value;render();return true;}
  if(name==='settings-apply'){applySettings();render();toast('Preferences applied in the prototype.');return true;}
  if(name==='settings-reset'){dialog('Restore settings defaults?',`<p>${state.settingsTab==='Audio'?'This resets only your draft audio volumes. Apply to keep them.':state.settingsTab==='Controller'?'This resets your draft controller and touch preferences. Apply to keep them.':'This resets your draft preferences, except the controller\'s. Apply to keep them.'}</p>`,button('Keep changes','close-dialog',{ghost:true})+button('Restore defaults','settings-confirm-reset',{primary:true}));return true;}
  if(name==='settings-confirm-reset'){if(state.settingsTab==='Audio'){for(const key of ['sound', 'effects', 'ambient', 'music', 'footsteps', 'landings', 'breaking', 'placing', 'eating', 'swimming', 'floating', 'pickups', 'tools', 'vitals'])state.settingsDraft[key]=settingsDefaults[key];}else{state.settingsDraft=clone(settingsDefaults);}settingsChanged();closeDialog();render();return true;}
  if(name==='settings-back'){
    if(state.settingsDirty)dialog('Keep your changes?','<p>You have unapplied settings. Apply them before leaving, or discard the draft.</p>',button('Keep editing','close-dialog',{ghost:true})+button('Discard','settings-discard')+button('Apply & return','settings-return',{primary:true}));
    else {state.settingsDraft=null;navigate(state.returnTo);}
    return true;
  }
  if(name==='settings-discard'){state.settingsDraft=null;state.settingsDirty=false;closeDialog();navigate(state.returnTo);return true;}
  if(name==='settings-return'){applySettings();state.settingsDraft=null;closeDialog();navigate(state.returnTo);return true;}
  if(name==='settings-rebind'||name==='settings-rebind-alt'){state.pendingConflict=null;state.rebinding=value;state.rebindingSlot=name==='settings-rebind'?'bindings':'altBindings';const bound=state.settingsDraft[state.rebindingSlot][value];dialog('Change binding',`<p><b>${esc(value)}</b> · ${name==='settings-rebind'?'Primary':'Alternative'}</p><p>Press a key or mouse button. Escape cancels.</p>`,(bound?button('Clear binding','settings-clear-binding',{ghost:true}):'')+button('Cancel','close-dialog'));return true;}
  if(name==='settings-rebind-pad'){dialog('Change binding',`<p><b>${esc(value)}</b> · Controller</p><p>Press a controller button. Menu or Esc cancels.</p>`,button('Cancel','close-dialog'));return true;}
  if(name==='settings-binding-reset'){const d=state.settingsDraft;d.bindings[value]=settingsDefaults.bindings[value];if(settingsDefaults.altBindings[value])d.altBindings[value]=settingsDefaults.altBindings[value];else delete d.altBindings[value];settingsChanged();render();return true;}
  if(name==='settings-clear-binding'){state.settingsDraft[state.rebindingSlot][state.rebinding]='';state.rebinding=null;settingsChanged();closeDialog();render();return true;}
  if(name==='settings-binding-group'){state.bindingGroup=value;render();return true;}
  if(name==='settings-replace-binding'){const {key,owners}=state.pendingConflict;const d=state.settingsDraft;for(const [slot,action] of owners)d[slot][action]='';d[state.rebindingSlot][state.rebinding]=key;state.pendingConflict=null;state.rebinding=null;settingsChanged();closeDialog();render();return true;}
  return false;
}

document.addEventListener('input',e=>{
  if(e.target.dataset.bind!=='bindingSearch')return;
  state.bindingSearch=e.target.value;
  if($('#binding-results'))$('#binding-results').innerHTML=bindingRows();
});

document.addEventListener('keydown',e=>{
  if(!state.rebinding||state.pendingConflict||!$('#dialog-root').children.length)return;
  e.preventDefault();e.stopImmediatePropagation();
  if(e.key==='Escape'){state.rebinding=null;closeDialog();return;}
  const key=e.key===' '?'Space':e.key.length===1?e.key.toUpperCase():e.key;
  const slot=state.rebindingSlot||'bindings',d=state.settingsDraft;
  const owners=['bindings','altBindings'].flatMap(s=>Object.entries(d[s]).filter(([action,binding])=>binding===key&&!(s===slot&&action===state.rebinding)).map(([action])=>[s,action]));
  if(owners.length){
    state.pendingConflict={key,owners};
    dialog('Binding already in use',`<p>This binding is assigned to ${owners.map(([,a])=>esc(a)).join(', ')}. Replace it?</p>`,button('Keep existing','close-dialog',{ghost:true})+button('Replace binding','settings-replace-binding',{primary:true}));
    return;
  }
  d[slot][state.rebinding]=key;
  state.rebinding=null;settingsChanged();closeDialog();render();
});
