function icon(name,size=20) {
  const paths={
    close:'<path d="m6 6 12 12M6 18 18 6"/>',
    info:'<circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7h.01"/>',
    search:'<circle cx="10.5" cy="10.5" r="7"/><path d="m16 16 5 5"/>',
    reverse:'<path d="m5 7-3 3 3 3M2 10h15M19 11l3 3-3 3M22 14H7"/>',
    line:'<path d="M3 12h18"/><rect x="3" y="9" width="6" height="6"/><rect x="15" y="9" width="6" height="6"/>',
    plane:'<path d="m12 4 10 7-10 7-10-7 10-7ZM7 7l10 7M7 14l10-7"/>',
    box:'<path d="m12 2 10 6-10 6L2 8l10-6ZM2 8v9l10 5 10-5V8M12 14v8"/>',
    layout:'<rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><path d="M14 17.5h7M17.5 14v7"/>',
    behavior:'<path d="M4 6h16M4 12h16M4 18h16"/><circle cx="8" cy="6" r="2"/><circle cx="16" cy="12" r="2"/><circle cx="10" cy="18" r="2"/>',
    keyboard:'<rect x="2" y="5" width="20" height="14" rx="2"/><path d="M6 9h.01M10 9h.01M14 9h.01M18 9h.01M6 13h.01M10 13h.01M14 13h.01M18 13h.01M7 16h10"/>',
    controller:'<path d="M7 7h10a4 4 0 0 1 4 3l1 7a2 2 0 0 1-3.4 1.7L16 16H8l-2.6 2.7A2 2 0 0 1 2 17l1-7a4 4 0 0 1 4-3Z"/><path d="M7 10v5M4.5 12.5h5M16 11h.01M19 14h.01"/>',
    display:'<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>',
    audio:'<path d="m11 4-6 5H2v6h3l6 5V4ZM15 8a6 6 0 0 1 0 8M18 5a10 10 0 0 1 0 14"/>',
    accessibility:'<circle cx="12" cy="4" r="2"/><path d="m4 9 8 2 8-2M12 11v5M12 16l-5 6M12 16l5 6"/>',
    storage:'<path d="M8 6V5a4 4 0 0 1 8 0v1M6 6h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2Z"/><path d="M8 13h8v6H8zM9 9h6"/>',
    catalogue:'<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>',
    world:'<circle cx="12" cy="12" r="9"/><ellipse cx="12" cy="12" rx="4" ry="9"/><path d="M3 12h18"/>',
    mods:'<path d="m12 3 9 5-9 5-9-5 9-5ZM3 8v9l9 5 9-5V8M12 13v9"/>',
    biomes:'<path d="M20 3C9 2 3 7 4 14c1 6 11 7 14 1 2-4 1-7 2-12Z"/><path d="M3 22 15 10"/>',
    review:'<rect x="5" y="4" width="14" height="18" rx="2"/><path d="M9 4V2h6v2M9 13l2 2 4-4"/>',
    chevron:'<path d="m9 5 7 7-7 7"/>',
    down:'<path d="m6 9 6 6 6-6"/>',
    check:'<path d="m5 12 4 4L19 6"/>'
  };
  return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${paths[name]||paths.behavior}</svg>`;
}

function customSelect(label,bind,value,options) {
  const items=options.map(x=>({value:String(x),label:bind==='settings.textScale'?Math.round(Number(x)*100)+'%':String(x)}));
  const selected=items.find(x=>x.value===String(value))?.label||String(value);
  return `<button type="button" class="select-trigger input" data-dropdown="${esc(bind)}" data-bind="${esc(bind)}" data-options="${esc(JSON.stringify(items))}" value="${esc(value)}" aria-label="${esc(label)}" aria-haspopup="listbox" aria-expanded="false"><span class="select-value">${esc(selected)}</span>${icon('down',16)}</button>`;
}

let openSelect=null;
function closeSelect(restoreFocus=false) {
  if(!openSelect)return;
  const trigger=openSelect.trigger;
  $('#select-layer')?.remove();
  trigger.setAttribute('aria-expanded','false');trigger.removeAttribute('aria-controls');
  openSelect=null;
  if(restoreFocus&&trigger.isConnected)trigger.focus({preventScroll:true});
}

function openDropdown(trigger,at) {
  closeSelect();
  trigger.focus({preventScroll:true});
  const items=JSON.parse(trigger.dataset.options);
  const index=at??Math.max(0,items.findIndex(x=>x.value===trigger.value));
  openSelect={trigger,items,index};
  const popup=document.createElement('div');
  popup.id='select-layer';popup.className='select-menu';popup.setAttribute('role','listbox');
  popup.setAttribute('aria-label',trigger.getAttribute('aria-label'));
  popup.innerHTML=items.map((item,i)=>`<button type="button" role="option" tabindex="-1" class="select-option" data-option="${i}" aria-selected="${item.value===trigger.value}"><span>${esc(item.label)}</span><span class="option-check">${item.value===trigger.value?icon('check',18):''}</span></button>`).join('');
  $('#game').append(popup);
  trigger.setAttribute('aria-expanded','true');trigger.setAttribute('aria-controls',popup.id);
  const rect=trigger.getBoundingClientRect(),game=$('#game').getBoundingClientRect();
  openSelect.anchor={x:rect.x,y:rect.y};
  const width=Math.min(Math.max(rect.width,190),game.width-24);
  popup.style.width=width+'px';
  popup.style.left=Math.max(12,Math.min(rect.left-game.left,game.width-width-12))+'px';
  const bottom=Math.min(game.bottom,innerHeight)-rect.bottom-12;
  const top=rect.top-Math.max(0,game.top)-12;
  const height=Math.min(popup.scrollHeight+2,280);
  const above=bottom<Math.min(height,150)&&top>bottom;
  const available=Math.max(48,above?top:bottom);
  popup.style.maxHeight=Math.min(280,available)+'px';
  popup.style.top=(above?rect.top-game.top-Math.min(height,available)-6:rect.bottom-game.top+6)+'px';
  focusSelectOption(index);
}

function focusSelectOption(index) {
  if(!openSelect)return;
  openSelect.index=(index+openSelect.items.length)%openSelect.items.length;
  const options=[...$('#select-layer').querySelectorAll('[role=option]')];
  options.forEach((el,i)=>el.classList.toggle('option-focused',i===openSelect.index));
  options[openSelect.index].focus({preventScroll:true});
  const option=options[openSelect.index],popup=$('#select-layer');
  if(option.offsetTop<popup.scrollTop)popup.scrollTop=option.offsetTop;
  else if(option.offsetTop+option.offsetHeight>popup.scrollTop+popup.clientHeight)popup.scrollTop=option.offsetTop+option.offsetHeight-popup.clientHeight;
}

function chooseSelect(index) {
  if(!openSelect)return;
  const {trigger,items}=openSelect,item=items[index];
  trigger.value=item.value;
  trigger.querySelector('.select-value').textContent=item.label;
  trigger.dispatchEvent(new Event('input',{bubbles:true}));
  trigger.dispatchEvent(new Event('change',{bubbles:true}));
  closeSelect(true);
}

document.addEventListener('click',e=>{
  const option=e.target.closest('[data-option]');
  if(option){chooseSelect(Number(option.dataset.option));return;}
  const trigger=e.target.closest?.('[data-dropdown]');
  if(trigger){openSelect?.trigger===trigger?closeSelect(true):openDropdown(trigger);return;}
  if(openSelect&&!e.target.closest('#select-layer'))closeSelect();
});

document.addEventListener('keydown',e=>{
  if(openSelect){
    if(e.key==='Tab'){
      const trigger=openSelect.trigger;closeSelect();trigger.focus({preventScroll:true});return;
    }
    e.preventDefault();e.stopImmediatePropagation();
    if(e.key==='Escape'){closeSelect(true);return;}
    if(e.key==='ArrowDown'){focusSelectOption(openSelect.index+1);return;}
    if(e.key==='ArrowUp'){focusSelectOption(openSelect.index-1);return;}
    if(e.key==='Home'){focusSelectOption(0);return;}
    if(e.key==='End'){focusSelectOption(openSelect.items.length-1);return;}
    if(e.key==='Enter'||e.key===' '){chooseSelect(openSelect.index);return;}
    if(e.key.length===1){const i=openSelect.items.findIndex(x=>x.label.toLowerCase().startsWith(e.key.toLowerCase()));if(i>=0)focusSelectOption(i);}
    return;
  }
  const trigger=e.target.closest?.('[data-dropdown]');
  if(trigger&&['ArrowDown','ArrowUp'].includes(e.key)){
    e.preventDefault();e.stopImmediatePropagation();
    openDropdown(trigger,e.key==='ArrowUp'?JSON.parse(trigger.dataset.options).length-1:undefined);
  }
});
window.addEventListener('resize',()=>closeSelect());
document.addEventListener('scroll',e=>{
  if(!openSelect||$('#select-layer')?.contains(e.target))return;
  const rect=openSelect.trigger.getBoundingClientRect();
  if(Math.abs(rect.x-openSelect.anchor.x)>1||Math.abs(rect.y-openSelect.anchor.y)>1)closeSelect();
},true);
