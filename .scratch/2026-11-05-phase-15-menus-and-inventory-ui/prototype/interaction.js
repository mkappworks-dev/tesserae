function savePrototype() {
  try {
    const {dialogFocus,rebinding,...saved}=state;
    sessionStorage.setItem('tesserae-cinematic-v1',JSON.stringify({state:saved,worlds}));
  } catch {}
}
function restorePrototype() {
  try {
    const saved=JSON.parse(sessionStorage.getItem('tesserae-cinematic-v1'));
    if(saved?.state&&Array.isArray(saved.worlds)){
      Object.assign(state,saved.state);worlds=saved.worlds;
      state.dialogFocus=null;state.rebinding=null;
    }
  } catch {}
}
function applyComfort() {
  const prefs=state.settings;
  $('#game').classList.toggle('no-transparency',!!prefs.simpleBackground);
  $('#game').style.setProperty('--text-scale',Number(prefs.textScale||1));
  $('#game').classList.toggle('large-text',Number(prefs.textScale||1)>=1.25);
}
function validationError(message,bind) {
  const body=$('.body');
  if(!body){toast(message);return false;}
  body.querySelector('.validation-summary')?.remove();
  const error=document.createElement('div');
  error.className='validation-summary';error.id='validation-error';error.setAttribute('role','alert');error.textContent=message;
  body.prepend(error);body.scrollTop=0;
  const control=bind?body.querySelector(`[data-bind="${bind}"]`):null;
  if(control){control.setAttribute('aria-invalid','true');control.setAttribute('aria-describedby','validation-error');control.focus();control.scrollIntoView({block:'nearest',behavior:'instant'});}
  return false;
}
function validateStep(step) {
  if(step===0&&state.name.length>64)return validationError('Use a World name of 64 characters or fewer.','name');
  if(step!==2)return true;
  if(!state.biomes.some(b=>b.enabled))return validationError('Enable at least one biome to continue.');
  for(const key of ['tempSize','humiditySize']){
    const number=Number(state[key]);
    if(!Number.isFinite(number)||number<=0||number>100000)return validationError('Climate sizes must be between 1 and 100,000 metres.',key);
  }
  for(let i=0;i<state.biomes.length;i++){
    const biome=state.biomes[i];
    for(const [key,maximum] of [['t',1],['h',1],['caves',4]]){
      if(!Number.isFinite(biome[key])||biome[key]<0||biome[key]>maximum){
        state.selected=i;render();
        return validationError(`${biome.name}: ${key==='caves'?'cave scale must be 0–4':'temperature and humidity must be 0–1'}.`,`biome.${key}`);
      }
    }
  }
  return true;
}
function menuTargets() {
  const root=openSelect?$('#select-layer'):$('#dialog-root').children.length?$('#dialog-root'):$('#game');
  return [...root.querySelectorAll('button:not(:disabled),input:not(:disabled),select:not(:disabled),summary')].filter(el=>el.getClientRects().length&&!el.closest('[inert]'));
}
function moveMenuFocus(direction) {
  const controls=menuTargets();if(!controls.length)return;
  const current=document.activeElement;
  const rect=current?.getBoundingClientRect();
  if(!controls.includes(current)){controls[0].focus();return;}
  const cx=rect.x+rect.width/2,cy=rect.y+rect.height/2;
  const candidates=controls.filter(el=>el!==current).map(el=>{
    const r=el.getBoundingClientRect(),dx=r.x+r.width/2-cx,dy=r.y+r.height/2-cy;
    const along=direction==='down'?dy:direction==='up'?-dy:direction==='right'?dx:-dx;
    const across=direction==='down'||direction==='up'?Math.abs(dx):Math.abs(dy);
    return {el,along,score:along+across*3};
  }).filter(x=>x.along>3).sort((a,b)=>a.score-b.score);
  if(candidates[0])candidates[0].el.focus();
}
function menuBack(){document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));}
let padLast=0,padHeld=false;
function handleMenuPad(pad,time) {
  if(!pad)return;
  const pressed=i=>!!pad.buttons[i]?.pressed;
  const active=[0,1,4,5,9,12,13,14,15].some(pressed);
  if(!active){padHeld=false;return;}
  const direction=pressed(12)?'up':pressed(13)?'down':pressed(14)?'left':pressed(15)?'right':null;
  if(padHeld&&(!direction||time-padLast<220))return;
  padLast=time;padHeld=true;
  if(openSelect){if(direction==='down')focusSelectOption(openSelect.index+1);else if(direction==='up')focusSelectOption(openSelect.index-1);else if(pressed(0))chooseSelect(openSelect.index);else if(pressed(1))closeSelect(true);return;}
  if(direction){const el=document.activeElement;if(el?.matches('input[type=range]')&&['left','right'].includes(direction)){const step=Number(el.step)||1;el.value=Number(el.value)+(direction==='right'?step:-step);el.dispatchEvent(new Event('input',{bubbles:true}));}else moveMenuFocus(direction);return;}
  if(pressed(1)){menuBack();return;}
  if(pressed(9)){state.screen==='play'?navigate('pause'):menuBack();return;}
  if(pressed(4)||pressed(5)){
    if(state.screen==='settings'){
      const i=settingsCategories.indexOf(state.settingsTab),n=pressed(5)?1:-1;
      action('settings-category',settingsCategories[(i+n+settingsCategories.length)%settingsCategories.length]);
    }else if(state.screen==='wizard')goStep(Math.min(3,Math.max(0,state.step+(pressed(5)?1:-1))));
    return;
  }
  if(pressed(0)){
    if(!menuTargets().includes(document.activeElement)){menuTargets()[0]?.focus();return;}
    const el=document.activeElement;
    if(el.tagName==='SELECT'){
      el.selectedIndex=(el.selectedIndex+1)%el.options.length;el.dispatchEvent(new Event('input',{bubbles:true}));
    }else el.click();
  }
}
function pollMenuPad(time){
  if(navigator.getGamepads)handleMenuPad([...navigator.getGamepads()].find(Boolean),time);
  requestAnimationFrame(pollMenuPad);
}
document.addEventListener('keydown',e=>{
  if(e.defaultPrevented||state.rebinding||!['ArrowUp','ArrowDown','ArrowLeft','ArrowRight'].includes(e.key))return;
  if(e.target.matches?.('input,select,textarea'))return;
  e.preventDefault();moveMenuFocus(e.key.slice(5).toLowerCase());
});
document.addEventListener('focusin',e=>{
  const scroller=e.target.closest('.body,.rail,.item-grid-scroll,.item-inspector,.layout-scroll');
  if(!scroller)return;
  const field=e.target.getBoundingClientRect(),box=scroller.getBoundingClientRect();
  if(field.top<box.top+8)scroller.scrollTop-=box.top+8-field.top;
  else if(field.bottom>box.bottom-8)scroller.scrollTop+=field.bottom-box.bottom+8;
});
document.addEventListener('input',e=>{if(e.target.closest('#game'))savePrototype();});
document.addEventListener('change',savePrototype);
window.addEventListener('beforeunload',savePrototype);
if(!document.documentElement.hasAttribute('data-embed'))requestAnimationFrame(pollMenuPad);

document.addEventListener('input',e=>{if(e.target.type!=='range'||!e.target.dataset.bind?.startsWith('biome.'))return;const out=document.querySelector(`output[data-for="${e.target.dataset.bind}"]`);if(!out)return;const v=Number(e.target.value);out.textContent=e.target.dataset.bind==='biome.caves'?(v===0?'Off':'× '+v.toFixed(1)):v.toFixed(2);});
