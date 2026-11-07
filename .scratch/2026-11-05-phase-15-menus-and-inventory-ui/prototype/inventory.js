const catalogueItems=[
  {id:'stone',itemId:'rock',name:'Stone',group:'Solids',color:'#969a97'},
  {id:'dirt',itemId:'dirt',name:'Dirt',group:'Solids',color:'#a18360'},
  {id:'grass',itemId:'grass',name:'Grass',group:'Solids',color:'#8ca968'},
  {id:'sand',itemId:'sand',name:'Sand',group:'Solids',color:'#d7c49c'},
  {id:'snow',itemId:'snow',name:'Snow',group:'Solids',color:'#e0e8e4'},
  {id:'iron',itemId:'iron',name:'Iron ore',group:'Solids',color:'#b59a85'},
  {id:'melon',itemId:'melon',name:'Melon',group:'Solids',color:'#54963a'},
  {id:'lamp',itemId:'lamp',name:'Lamp',group:'Solids',color:'#efce8d'},
  {id:'water',itemId:'water',name:'Water source',group:'Fluids',color:'#75a9be'},
  {id:'lava',itemId:'lava',name:'Lava source',group:'Fluids',color:'#dc9160'},
  {id:'mud',itemId:'mud',name:'Mud source',group:'Fluids',color:'#96866b'}
];
const carriedItems=catalogueItems.map(x=>({...x,id:x.itemId,name:x.itemId==='rock'?'Rock':x.name,material:x.name,materialId:x.id,maxStack:64}));
function creativeAccess(){return state.itemAccess!=='survival';}
function stackItem(stack){return stack?carriedItems.find(x=>x.id===stack.id):null;}
function stackKey(stack){return stack.id+':'+(stack.shape||'Cube');}
function ensureInventory(){
  for(const prefs of [state.settings,state.settingsDraft].filter(Boolean))if(prefs.bindings?.['Self storage']){
    prefs.bindings.Inventory=prefs.bindings['Self storage'];delete prefs.bindings['Self storage'];
  }
  if(state.itemKeymapRevision!==2){
    for(const prefs of [state.settings,state.settingsDraft].filter(Boolean)){
      prefs.bindings ||= {};
      if(prefs.bindings.Catalogue==='E')prefs.bindings.Catalogue='C';
      if(prefs.bindings.Crouch==='C')prefs.bindings.Crouch='Control';
      if(prefs.bindings.Menu==='Escape')prefs.bindings.Menu='Tab';
      for(const [name,key] of Object.entries({'Inventory':'I','Back / menu':'Escape','Settings':'F10','Flip':'E'}))prefs.bindings[name] ??=key;
    }
    state.itemKeymapRevision=2;
  }
  state.itemAccess ||= 'creative';
  state.bag ||= Array.from({length:27},(_,i)=>i===0?{id:'rock',count:48}:i===1?{id:'dirt',count:24}:i===2?{id:'grass',count:16}:null);
  state.bagHotbar ||= Array.from({length:9},()=>null);
  if(!state.sharedBarRevision){
    if(!state.bagHotbar.some(Boolean))for(let i=0;i<3;i++)if(state.bag[i]){
      const count=Math.min(state.bag[i].count,8);
      state.bagHotbar[i]={...state.bag[i],count};state.bag[i].count-=count;
      if(!state.bag[i].count)state.bag[i]=null;
    }
    state.sharedBarRevision=1;
  }
  state.catalogueItem ||= 'stone';state.catalogueShape ||= 'Cube';state.catalogueQuery ||= '';state.catalogueGroup ||= 'All';state.quickSlot ??=0;state.sideItem ||= 'stone';state.sideGroup ||= 'All';state.transferCount ??= 64;
  state.inspected ||= {area:'bag',index:0};
}
function voxelIcon(item,shape='Cube'){
  if(!item)return '';
  const top=shape==='Slab'?'m24 14 19 11-19 11L5 25Z':'m24 3 19 11-19 11L5 14Z';
  const left=shape==='Slab'?'m5 25 19 11v9L5 34Z':'m5 14 19 11v20L5 34Z';
  const right=shape==='Slab'?'m24 36 19-11v9L24 45Z':'m24 25 19-11v20L24 45Z';
  if(shape==='Slope')return `<svg class="voxel-icon" viewBox="0 0 48 48" aria-hidden="true"><path d="m5 34 19-31 19 11-19 31Z" fill="${item.color}"/><path d="M5 34 24 3v42Z" fill="${item.color}"/><path d="m24 45 19-31v20Z" fill="${item.color}"/><path d="M5 34 24 3v42Z" fill="#000" opacity=".15"/><path d="m24 45 19-31v20Z" fill="#000" opacity=".35"/></svg>`;
  return `<svg class="voxel-icon" viewBox="0 0 48 48" aria-hidden="true"><path d="${top}" fill="${item.color}"/><path d="${left}" fill="${item.color}"/><path d="${right}" fill="${item.color}"/><path d="${left}" fill="#000" opacity=".16"/><path d="${right}" fill="#000" opacity=".32"/></svg>`;
}
function inventoryTabs(){return `<div class="inventory-title">${icon('storage')}<h1>Inventory</h1><kbd>I</kbd></div>`;}
function itemShell(content,footer){return `<section class="item-window" data-details="${!!state.showItemDetails}" aria-label="Player inventory"><header class="item-window-header">${inventoryTabs()}<div class="item-window-context"><button class="item-details-toggle" data-action="item-details" aria-label="Toggle item details" aria-expanded="${!!state.showItemDetails}" aria-controls="item-inspector">${icon('info')}</button>${creativeAccess()?'<span class="access-badge">Creative access</span>':''}<button class="item-close" data-action="inventory-close" aria-label="Close and return to game">${icon('close')}</button></div></header>${content}<footer class="item-window-footer">${footer}</footer></section>`;}
function renderCarriedSlot(area,index,play=false){
  const stack=(area==='bag'?state.bag:state.bagHotbar)[index],item=stackItem(stack);
  const picked=state.carriedFrom?.area===area&&state.carriedFrom?.index===index;
  const inspected=state.inspected?.area===area&&state.inspected?.index===index;
  return `<button class="item-slot ${picked?'picked-up':''} ${!play&&inspected?'inspected':''} ${area==='bar'&&index===state.quickSlot?'active':''}" data-action="${play?'play-slot':'storage-slot'}" data-value="${play?index:area+':'+index}" aria-label="${area==='bag'?'Inventory':'Hotbar'} slot ${index+1}${stack?': '+item?.name+', '+stack.count+', '+(stack.shape||'Cube'):' empty'}" ${play?'':`draggable="${!!stack}" data-drag-area="${area}" data-drag-index="${index}"`}>${voxelIcon(item,stack?.shape)}${stack?`<span class="stack-count">${stack.count}</span>`:''}${area==='bar'?`<span class="slot-number">${index+1}</span>`:''}</button>`;
}
function creativeBar(){return state.bagHotbar.map((_,i)=>renderCarriedSlot('bar',i,true)).join('');}
function itemHotbar(){return `<div class="item-hotbar"><div class="hotbar-caption"><b>Hotbar</b><small>Quick access · the same slots you use in game</small></div><div class="storage-grid quick-access">${state.bagHotbar.map((_,i)=>renderCarriedSlot('bar',i)).join('')}</div></div>`;}
function storageInspector(){
  const from=state.carriedFrom||state.inspected;
  const stack=from?(from.area==='bag'?state.bag:state.bagHotbar)[from.index]:null,item=stackItem(stack);
  return `<aside class="item-inspector" id="item-inspector"><button class="item-inspector-close" data-action="item-details" data-value="close" aria-label="Close item details">${icon('close')}</button><p class="item-eyebrow">${state.carriedFrom?'MOVING STACK':'ITEM DETAILS'}</p>${item?`${voxelIcon(item,stack.shape)}<h2>${item.name}</h2><p class="item-source">terra:${item.id}${item.materialId!==item.id?` · places ${item.material}`:''}</p><dl><div><dt>Count</dt><dd>${stack.count}</dd></div><div><dt>Shape</dt><dd>${stack.shape||'Cube'}</dd></div><div><dt>Stack limit</dt><dd>${item.maxStack}</dd></div></dl><p class="section-label">Shape · ${esc(item.name)}</p><div class="row wrap">${['Cube','Post','Slab','Stair','Fence'].map(s=>button(s,'inventory-shape',{value:s,small:true,ghost:(state.placeShape||'Cube')!==s})).join('')}</div><div class="spacer"></div><div class="inspector-actions">${button('Move to '+(from.area==='bag'?'hotbar':'backpack'),'storage-quick',{value:from.area+':'+from.index})}${button('Split stack','storage-split',{value:from.area+':'+from.index,disabled:stack.count<2})}${button('Drop 1','storage-drop-one',{value:from.area+':'+from.index})}${button('Drop stack','storage-drop-all',{value:from.area+':'+from.index})}${state.carriedFrom?button('Cancel move','storage-cancel',{ghost:true}):''}</div>`:`<div class="empty-item">${icon('storage',40)}</div><h2>Choose an item</h2><p>Select a carried stack to inspect or move it.</p>`}<div class="inspector-note">${state.carriedFrom?'Choose another slot to move, merge or swap this stack.':'Shift-click moves a stack between your backpack and hotbar.'}</div></aside>`;
}
function storagePage(){
  ensureInventory();
  const main=`<div class="item-main"><div class="item-browser inventory-browser"><div class="inventory-heading"><div><p class="item-eyebrow">YOUR ITEMS</p><h1>Backpack</h1></div><div class="row"><span class="item-capacity ${state.bag.every(Boolean)?'full':''}">${state.bag.every(Boolean)?'Full · ':''}${state.bag.filter(Boolean).length} / ${state.bag.length} slots</span>${button('Sort','storage-sort',{small:true,ghost:true})}</div></div><div class="item-grid-scroll"><div class="storage-grid backpack-grid">${state.bag.map((_,i)=>renderCarriedSlot('bag',i)).join('')}</div><div class="inventory-inline-hint" role="status">${state.carriedFrom?'Stack selected. Choose a destination, or select the source again to cancel.':'Click to move · Shift-click for quick transfer · Drag and drop supported · Drag out to drop'}</div></div></div>${storageInspector()}</div>`;
  const win=itemShell(main,`${itemHotbar()}<div class="item-footer-actions"><small>${creativeAccess()?'Drag from the Catalogue into any slot, or set a count and transfer.':'Item counts stay intact when switching screens.'}</small>${button('Back to game','inventory-close',{ghost:true})}</div>`);
  return `<div class="inventory-overlay ${creativeAccess()?'with-catalogue':''}">${creativeAccess()?catalogueSide():''}${win}</div>`;
}
const catalogueGroups=['All','Solids','Fluids'];
function sideItems(){const q=(state.quickAddQuery||'').toLowerCase(),g=catalogueGroups.includes(state.sideGroup)?state.sideGroup:'All';return catalogueItems.filter(x=>(g==='All'||x.group===g)&&x.name.toLowerCase().includes(q));}
function sideGrid(){const items=sideItems();return items.length?items.map(item=>`<button class="catalogue-tile side-tile ${item.id===state.sideItem?'selected':''}" draggable="true" data-catalogue-drag="${item.id}" data-action="side-pick" data-value="${item.id}" aria-pressed="${item.id===state.sideItem}" title="${item.name}: drag to a slot · double-click for hotbar slot ${state.quickSlot+1} · Shift-click for backpack">${voxelIcon(item)}<b>${item.name}</b></button>`).join(''):'<p class="quick-add-empty">No Materials match.</p>';}
function catalogueSide(){
  const item=catalogueItems.find(x=>x.id===state.sideItem)||catalogueItems[0],n=state.transferCount;
  return `<aside class="catalogue-side" aria-label="Catalogue"><header><p class="item-eyebrow">CREATIVE</p><h2>Catalogue</h2><small>Drag a Material into any slot</small></header><div class="catalogue-search side-search">${icon('search',16)}<input class="input" data-bind="quickAddQuery" value="${esc(state.quickAddQuery||'')}" placeholder="Search Materials…" aria-label="Search Catalogue"></div><div class="side-filters" aria-label="Catalogue category">${catalogueGroups.map(g=>`<button data-action="side-group" data-value="${g}" class="${(catalogueGroups.includes(state.sideGroup)?state.sideGroup:'All')===g?'active':''}" aria-pressed="${(catalogueGroups.includes(state.sideGroup)?state.sideGroup:'All')===g}">${g}</button>`).join('')}</div><div class="side-grid" id="side-grid">${sideGrid()}</div><footer class="transfer"><div class="transfer-item">${voxelIcon(item)}<span><b>${item.name}</b><small>terra:${item.id}</small></span></div><div class="transfer-count"><span>Count</span><div class="layout-stepper"><button data-action="transfer-count" data-value="-1" aria-label="One fewer">−</button><input class="input" type="number" min="1" max="64" inputmode="numeric" data-bind="transferCount" value="${n}" aria-label="Count to transfer"><button data-action="transfer-count" data-value="1" aria-label="One more">+</button></div><div class="count-presets">${[1,16,64].map(v=>`<button data-action="transfer-preset" data-value="${v}" class="${n===v?'active':''}">${v}</button>`).join('')}</div></div><div class="transfer-actions">${button('To backpack','transfer-bag',{ghost:true})}${button(`To hotbar ${state.quickSlot+1}`,'transfer-bar',{primary:true})}</div></footer></aside>`;
}
function giveToBag(id,count){
  const item=catalogueItems.find(x=>x.id===id),next=stashStack(state.bag,{id:item.itemId,count,shape:'Cube'});
  if(!next){toast('Backpack full. Make room first.');return false;}
  state.bag=next;toast(`${count} ${item.name} added to the backpack.`);return true;
}
function giveToSlot(area,index,id,count){
  const item=catalogueItems.find(x=>x.id===id),list=area==='bag'?state.bag:state.bagHotbar,stack={id:item.itemId,count,shape:'Cube'},here=list[index];
  if(here&&stackKey(here)===stackKey(stack)){here.count=Math.min(stackItem(here).maxStack,here.count+count);}
  else{
    if(here){list[index]=null;const next=stashStack(state.bag,here);if(!next){list[index]=here;toast('Backpack full. Make room for the stack in this slot first.');return false;}state.bag=next;}
    list[index]=stack;
  }
  toast(`${item.name} × ${list[index].count} in ${area==='bag'?'backpack':'hotbar'} slot ${index+1}.`);return true;
}
function catalogueGive(target,id=state.sideItem,count=state.transferCount){
  if(!creativeAccess()){toast('The Catalogue is read-only in Survival preview.');return;}
  const from=$(`.side-tile[data-value="${id}"]`)?.getBoundingClientRect(),before=JSON.stringify(state.bag);
  const done=target==='bag'?giveToBag(id,count):giveToSlot(target.area,target.index,id,count);
  if(done){
    const at=target==='bag'?{area:'bag',index:state.bag.findIndex((s,i)=>JSON.stringify(s)!==JSON.stringify(JSON.parse(before)[i]))}:target;
    state.carriedFrom=null;refreshInventory();savePrototype();flyItem(from,at,id);
  }
}
/* A visual only: the stacks above are already final when this runs. */
function flyItem(from,at,id){
  if(!from||state.settings.motion||at.index<0)return;
  const slot=$(`.item-window [data-drag-area="${at.area}"][data-drag-index="${at.index}"],.item-hotbar [data-drag-area="${at.area}"][data-drag-index="${at.index}"]`);
  const item=catalogueItems.find(x=>x.id===id);
  if(!slot||!item)return;
  const to=slot.getBoundingClientRect(),size=Math.min(to.width,to.height)*.62;
  const fly=document.createElement('div');fly.className='item-flight';fly.setAttribute('aria-hidden','true');fly.innerHTML=voxelIcon(item);
  Object.assign(fly.style,{width:size+'px',height:size+'px',left:from.left+from.width/2-size/2+'px',top:from.top+from.height/2-size/2+'px'});
  document.body.append(fly);
  const dx=to.left+to.width/2-(from.left+from.width/2),dy=to.top+to.height/2-(from.top+from.height/2);
  fly.animate([{transform:'translate(0,0) scale(1)',opacity:1},{transform:`translate(${dx}px,${dy}px) scale(.8)`,opacity:1,offset:.85},{transform:`translate(${dx}px,${dy}px) scale(.8)`,opacity:0}],{duration:250,easing:'ease-out',fill:'forwards'}).finished.then(()=>fly.remove(),()=>fly.remove());
}
function refreshInventory(){
  const root=$('.item-grid-scroll'),y=root?.scrollTop||0,focused=document.activeElement;
  const actionName=focused?.dataset.action,value=focused?.dataset.value;
  render();if($('.item-grid-scroll'))$('.item-grid-scroll').scrollTop=y;
  const next=[...document.querySelectorAll('.item-window [data-action]')].find(x=>x.dataset.action===actionName&&x.dataset.value===value);
  next?.focus({preventScroll:true});
}
function stashStack(list,stack){
  const copy=clone(list),remaining={...stack};
  for(const target of copy)if(target&&stackKey(target)===stackKey(remaining)){
    const count=Math.min(remaining.count,stackItem(target).maxStack-target.count);target.count+=count;remaining.count-=count;if(!remaining.count)return copy;
  }
  for(let i=0;i<copy.length;i++)if(!copy[i]){const count=Math.min(remaining.count,stackItem(remaining).maxStack);copy[i]={...remaining,count};remaining.count-=count;if(!remaining.count)return copy;}
  return null;
}
function storageMove(area,index){
  ensureInventory();const target=area==='bag'?state.bag:state.bagHotbar;
  state.inspected={area,index};if(area!=='bag')state.quickSlot=index;
  if(!state.carriedFrom){if(target[index])state.carriedFrom={area,index};refreshInventory();return;}
  const source=state.carriedFrom.area==='bag'?state.bag:state.bagHotbar,from=state.carriedFrom.index;
  if(source===target&&from===index){state.carriedFrom=null;refreshInventory();return;}
  if(target[index]&&source[from]&&stackKey(target[index])===stackKey(source[from])){
    const count=Math.min(source[from].count,stackItem(target[index]).maxStack-target[index].count);
    if(!count){toast('That stack is full. Choose another slot.');return;}
    target[index].count+=count;source[from].count-=count;if(!source[from].count)source[from]=null;
  }else [source[from],target[index]]=[target[index],source[from]];
  state.carriedFrom=null;refreshInventory();savePrototype();
}
function quickTransfer(area,index){
  const source=area==='bag'?state.bag:state.bagHotbar,stack=source[index];if(!stack)return;
  const moved=stashStack(area==='bag'?state.bagHotbar:state.bag,stack);
  if(!moved){toast('Not enough room for this stack.');return;}
  if(area==='bag')state.bagHotbar=moved;else state.bag=moved;
  source[index]=null;state.carriedFrom=null;refreshInventory();
}
function inventoryAction(name,value){
  if(name==='inventory-shape'){state.placeShape=value;render();return true;}
  if(name==='item-details'){state.showItemDetails=value==='close'?false:!state.showItemDetails;refreshInventory();return true;}
  if(name==='inventory-open'){ensureInventory();state.carriedFrom=null;navigate('storage');return true;}
  if(name==='catalogue-open'){
    ensureInventory();state.carriedFrom=null;
    if(!creativeAccess())toast('The Catalogue needs creative access.');
    if(state.screen!=='storage')navigate('storage');
    $('.side-search input')?.focus();return true;
  }
  if(name==='inventory-close'){state.carriedFrom=null;navigate('play');return true;}
  if(name==='storage-slot'){const [area,index]=value.split(':');storageMove(area,Number(index));return true;}
  if(name==='storage-cancel'){state.carriedFrom=null;refreshInventory();return true;}
  if(name==='storage-quick'){const [area,index]=value.split(':');quickTransfer(area,Number(index));return true;}
  if(name==='storage-drop-one'||name==='storage-drop-all'){const [area,index]=value.split(':');dropFrom(area,Number(index),name==='storage-drop-all');return true;}
  if(name==='storage-split'){
    const [area,index]=value.split(':'),source=area==='bag'?state.bag:state.bagHotbar,stack=source[Number(index)],empty=state.bag.findIndex(x=>!x);
    if(!stack||stack.count<2)return true;if(empty<0){toast('An empty backpack slot is needed to split this stack.');return true;}
    const count=Math.floor(stack.count/2);state.bag[empty]={...stack,count};stack.count-=count;state.carriedFrom=null;refreshInventory();return true;
  }
  if(name==='storage-sort'){
    const sorted=state.bag.filter(Boolean).sort((a,b)=>stackItem(a).name.localeCompare(stackItem(b).name)||stackKey(a).localeCompare(stackKey(b)));
    let list=Array(27).fill(null);for(const stack of sorted)list=stashStack(list,stack);
    state.bag=list;state.carriedFrom=null;state.inspected={area:'bag',index:0};refreshInventory();return true;
  }
  if(name==='play-slot'){const was=state.quickSlot;state.quickSlot=Number(value);if(was!==state.quickSlot&&state.screen==='play')handFx('switch');refreshInventory();return true;}
  if(name==='side-pick'){state.sideItem=value;refreshInventory();return true;}
  if(name==='side-group'){state.sideGroup=value;refreshInventory();return true;}
  if(name==='transfer-count'){state.transferCount=Math.max(1,Math.min(64,state.transferCount+Number(value)));refreshInventory();return true;}
  if(name==='transfer-preset'){state.transferCount=Number(value);refreshInventory();return true;}
  if(name==='transfer-bag'){catalogueGive('bag');return true;}
  if(name==='transfer-bar'){catalogueGive({area:'bar',index:state.quickSlot});return true;}
  return false;
}
document.addEventListener('click',e=>{
  const tile=e.target.closest('[data-action=side-pick]');
  if(tile&&e.shiftKey){e.preventDefault();e.stopImmediatePropagation();state.sideItem=tile.dataset.value;catalogueGive('bag');return;}
  const slot=e.target.closest('[data-action=storage-slot]');if(!slot||!e.shiftKey)return;
  e.preventDefault();e.stopImmediatePropagation();const [area,index]=slot.dataset.value.split(':');quickTransfer(area,Number(index));
});
document.addEventListener('change',e=>{if(e.target.dataset.bind!=='transferCount')return;e.stopImmediatePropagation();const v=Math.round(Number(e.target.value));state.transferCount=Number.isFinite(v)?Math.max(1,Math.min(64,v)):64;refreshInventory();savePrototype();});
document.addEventListener('input',e=>{
  const key=e.target.dataset.bind;
  if(key==='quickAddQuery'){state.quickAddQuery=e.target.value;$('#side-grid').innerHTML=sideGrid();savePrototype();e.stopImmediatePropagation();}
  if(key==='transferCount'){e.stopImmediatePropagation();}
  if(key==='itemAccess'){
    state.itemAccess=e.target.value;const w=activeWorld();if(w)w.playerMode=e.target.value==='survival'?'Survival':'Creative';state.carriedFrom=null;render();savePrototype();e.stopImmediatePropagation();
  }
});
document.addEventListener('keydown',e=>{
  if(e.defaultPrevented||openSelect||state.rebinding||$('#dialog-root').children.length)return;
  const typing=e.target.matches?.('input,textarea,select')||e.target.isContentEditable;
  if(typing&&e.key!=='Escape')return;
  if(state.screen==='play'&&e.key==='Tab'){e.preventDefault();e.stopImmediatePropagation();navigate('pause');return;}
  if(!['play','pause','storage'].includes(state.screen))return;
  if(e.key==='Escape'&&state.screen==='storage'){e.preventDefault();e.stopImmediatePropagation();action('inventory-close');return;}
  const key=e.key.toLowerCase();
  if(['i','c'].includes(key)&&state.screen!=='pause'){
    e.preventDefault();e.stopImmediatePropagation();action(key==='c'?'catalogue-open':state.screen==='storage'?'inventory-close':'inventory-open');return;
  }
  if(e.key==='F10'){e.preventDefault();e.stopImmediatePropagation();action('settings');return;}
  if(/^[1-9]$/.test(e.key)&&state.screen==='play'){
    e.preventDefault();ensureInventory();const was=state.quickSlot;state.quickSlot=Number(e.key)-1;if(was!==state.quickSlot&&state.screen==='play')handFx('switch');refreshInventory();
  }
});
document.addEventListener('dblclick',e=>{const tile=e.target.closest('[data-action=side-pick]');if(!tile)return;e.preventDefault();state.sideItem=tile.dataset.value;catalogueGive({area:'bar',index:state.quickSlot});});
document.addEventListener('dragstart',e=>{
  const source=e.target.closest('[data-catalogue-drag]');
  if(source){state.dragCatalogue=source.dataset.catalogueDrag;state.sideItem=state.dragCatalogue;e.dataTransfer.effectAllowed='copy';e.dataTransfer.setData('text/plain','catalogue:'+state.dragCatalogue);return;}
  const slot=e.target.closest('[data-drag-area]');if(!slot)return;
  state.carriedFrom={area:slot.dataset.dragArea,index:Number(slot.dataset.dragIndex)};e.dataTransfer.setData('text/plain',slot.dataset.dragArea+':'+slot.dataset.dragIndex);
});
document.addEventListener('dragover',e=>{if(e.target.closest('[data-drag-area]')||(state.dragCatalogue&&e.target.closest('.backpack-grid,.quick-access'))){e.preventDefault();if(state.dragCatalogue){e.dataTransfer.dropEffect='copy';e.target.closest('[data-drag-area]')?.classList.add('drop-target');}}});
document.addEventListener('dragleave',e=>{e.target.closest?.('[data-drag-area]')?.classList.remove('drop-target');});
document.addEventListener('drop',e=>{if(state.dragCatalogue){const id=state.dragCatalogue,slot=e.target.closest('[data-drag-area]');state.dragCatalogue=null;if(!slot&&!e.target.closest('.backpack-grid,.quick-access'))return;e.preventDefault();catalogueGive(slot?{area:slot.dataset.dragArea,index:Number(slot.dataset.dragIndex)}:'bag',id);return;}const slot=e.target.closest('[data-drag-area]');if(!slot||!state.carriedFrom)return;e.preventDefault();storageMove(slot.dataset.dragArea,Number(slot.dataset.dragIndex));});
/* Throws Items onto the ground, in either mode: the Server takes them out of the Stack and a toast says what went. */
function dropFrom(area,index,all){
  const source=area==='bag'?state.bag:state.bagHotbar,stack=source[index];if(!stack)return;
  const n=all?stack.count:1,name=stackItem(stack).name;stack.count-=n;if(!stack.count)source[index]=null;
  state.carriedFrom=null;state.inspected=null;toast(`Dropped ${n} ${name}.`);refreshInventory();
}
document.addEventListener('dragend',e=>{
  /* Let go over the World, outside the panels, and the whole Stack is thrown out. */
  const over=document.elementFromPoint(e.clientX,e.clientY);
  if(state.carriedFrom&&state.screen==='storage'&&e.dataTransfer?.dropEffect==='none'&&!over?.closest('.item-browser,.catalogue-side,.item-inspector,.item-header')){const {area,index}=state.carriedFrom;state.carriedFrom=null;dropFrom(area,index,true);return;}
  state.dragCatalogue=null;if(state.carriedFrom){state.carriedFrom=null;if(state.screen==='storage')refreshInventory();}});
