const settingsDefaults = {
  sprint:'Hold', removal:'Hold', repeat:true, mouse:1, invert:false,
  scroll:'Standard', scrollSpeed:1, reference:true, padLook:1,
  padInvert:false, moveDeadzone:.15, lookDeadzone:.12, crouch:'Hold',
  touchLook:1, touchInvert:false, touchLayout:'Normal', touchSize:'Normal',
  touchOpacity:.8, sync:'Display synced', aa:'FXAA', renderScale:'100%',
  fps:'120', sound:70, music:50, effects:80, motion:false, contrast:false, simpleBackground:false, textScale:"1",
  bindings:{'Move forward':'W','Move backward':'S','Move left':'A','Move right':'D',
    'Jump':'Space','Sprint':'Shift','Crouch':'Control','Place':'Mouse right',
    'Remove':'Mouse left','Catalogue':'C','Inventory':'I','Flip':'E','Layout':'L','Menu':'Tab','Back / menu':'Escape','Settings':'F10'}
};
const settingsCategories = ['Behavior','Key bindings','Controller','Display','Audio','Accessibility'];

function ensureSettings() {
  if (!state.settingsDraft) {
    state.settings={...clone(settingsDefaults),...state.settings};
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
      row('Sprint activation','sprint','select',['Hold','Toggle'])+
      row('Removal activation','removal','select',['Hold','Timed tap'])+
      row('Repeat while held','repeat','toggle',[],'Keep placing or removing while the binding is held.')+
      group('Mouse & quick slots')+
      row('Mouse sensitivity','mouse','range',[.1,3,.1],'Relative to the default mouse feel.')+
      row('Invert vertical look','invert','toggle')+
      row('Scroll direction','scroll','select',['Standard','Reversed'])+
      row('Scroll sensitivity','scrollSpeed','range',[.1,3,.1])+
      row('Show quick reference','reference','toggle');
    case 'Key bindings': return `<div class="row between"><span class="muted">Keyboard & mouse</span><span class="pill">Select to rebind</span></div>`+
      Object.entries(state.settingsDraft.bindings).map(([name,key])=>`<div class="setting-row"><b>${name}</b>${button(esc(key),'settings-rebind',{value:name})}</div>`).join('');
    case 'Controller': return group('Sticks & buttons')+
      row('Look sensitivity','padLook','range',[.1,3,.1])+
      row('Invert vertical look','padInvert','toggle')+
      row('Movement deadzone','moveDeadzone','range',[0,.5,.01],'Ignore small movements caused by stick drift.')+
      row('Look deadzone','lookDeadzone','range',[0,.5,.01])+
      row('Crouch activation','crouch','select',['Hold','Toggle'])+
      `<details><summary>Touch controls</summary>`+
      row('Look sensitivity','touchLook','range',[.1,3,.1])+
      row('Invert vertical look','touchInvert','toggle')+
      row('Layout','touchLayout','select',['Normal','Mirrored'])+
      row('Control size','touchSize','select',['Small','Normal','Enlarged'])+
      row('Control opacity','touchOpacity','range',[.2,1,.05])+`</details>`;
    case 'Display': return group('Frame presentation')+
      row('Display sync','sync','select',['Display synced','Sync off'],'Match frame presentation to your display.')+
      row('Frame rate limit','fps','select',['60','90','120','144','240','No limit'],'An optional ceiling, independent of display sync.')+
      group('Image quality')+
      row('Anti-aliasing','aa','select',['Off','FXAA','MSAA 4x'],'Smooth voxel edges. MSAA costs more.')+
      row('Render scale','renderScale','select',['50%','67%','75%','85%','100%'],'Lower values reduce rendering work. Menus stay sharp.');
    case 'Audio': return `<div class="row between"><p class="section-label">Sound mix</p><span class="pill">Proposed category</span></div>`+
      row('Master volume','sound','range',[0,100,1])+
      row('Music','music','range',[0,100,1])+
      row('Sound effects','effects','range',[0,100,1]);
    case 'Accessibility': return `<div class="row between"><p class="section-label">Comfort & readability</p><span class="pill">Proposed category</span></div>`+
      row('Reduced motion','motion','toggle',[],'Use immediate interface transitions.')+
      row('Stronger UI contrast','contrast','toggle',[],'Increase panel borders and secondary text contrast.')+
      row('Simple menu background','simpleBackground','toggle',[],'Replace the scenery with a solid background.')+
      row('Text size','textScale','select',['1','1.25','1.5','2'],'1 is standard; 2 doubles interface text. Layouts adapt.');
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
  if(name==='settings-reset'){dialog('Restore settings defaults?','<p>This resets your draft preferences across all categories. Apply to keep them.</p>',button('Keep changes','close-dialog',{ghost:true})+button('Restore defaults','settings-confirm-reset',{primary:true}));return true;}
  if(name==='settings-confirm-reset'){state.settingsDraft=clone(settingsDefaults);settingsChanged();closeDialog();render();return true;}
  if(name==='settings-back'){
    if(state.settingsDirty)dialog('Keep your changes?','<p>You have unapplied settings. Apply them before leaving, or discard the draft.</p>',button('Keep editing','close-dialog',{ghost:true})+button('Discard','settings-discard')+button('Apply & return','settings-return',{primary:true}));
    else {state.settingsDraft=null;navigate(state.returnTo);}
    return true;
  }
  if(name==='settings-discard'){state.settingsDraft=null;state.settingsDirty=false;closeDialog();navigate(state.returnTo);return true;}
  if(name==='settings-return'){applySettings();state.settingsDraft=null;closeDialog();navigate(state.returnTo);return true;}
  if(name==='settings-rebind'){state.rebinding=value;dialog('Change binding',`<p><b>${esc(value)}</b></p><p>Press a key. Escape cancels.</p>`,button('Cancel','close-dialog'));return true;}
  return false;
}

document.addEventListener('keydown',e=>{
  if(!state.rebinding||!$('#dialog-root').children.length)return;
  e.preventDefault();e.stopImmediatePropagation();
  if(e.key==='Escape'){state.rebinding=null;closeDialog();return;}
  const key=e.key===' '?'Space':e.key.length===1?e.key.toUpperCase():e.key;
  const conflict=Object.entries(state.settingsDraft.bindings).find(([action,binding])=>action!==state.rebinding&&binding===key);
  if(conflict){toast(`${key} is already used for ${conflict[0]}. Choose another key.`);return;}
  state.settingsDraft.bindings[state.rebinding]=key;
  state.rebinding=null;settingsChanged();closeDialog();render();
});
