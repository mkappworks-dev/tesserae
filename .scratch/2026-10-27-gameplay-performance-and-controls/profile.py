import os,subprocess,time,sys,json,hashlib,fcntl
from pathlib import Path
root=Path(__file__).resolve().parents[2]
out=Path(sys.argv.pop(1)).resolve();out.mkdir(parents=True,exist_ok=True)
lock=(out/'benchmark.lock').open('w');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
binary=Path(os.environ.get('TESSERAE_RESEARCH_BINARY',str(out/'tesserae-profile')))
if not binary.exists():
 import shutil
 shutil.copy2(str(Path(os.environ['CARGO_TARGET_DIR'])/'debug/tesserae'),binary)
print('binary_sha256',hashlib.sha256(binary.read_bytes()).hexdigest(),flush=True)
variants={'baseline':{},'still':{'TESSERAE_SCREENSHOT_SPEED_M_S':'0'},'atlas_frozen':{'TESSERAE_RESEARCH_FREEZE_ATLAS':'1'},'low_res':{'TESSERAE_SCREENSHOT_WIDTH':'960','TESSERAE_SCREENSHOT_HEIGHT':'540'},'menu':{'TESSERAE_SCREENSHOT_MENU':'1'},'no_shadows':{'TESSERAE_RESEARCH_NO_SHADOWS':'1'},'no_msaa':{'TESSERAE_RESEARCH_NO_MSAA':'1'},'no_lod':{'TESSERAE_LOD':'0'},'diagnostics':{'TESSERAE_RESEARCH_DIAGNOSTICS':'1'},'sample':{},'metal':{},'experimental':{'TESSERAE_SCREENSHOT_WIDTH':'960','TESSERAE_SCREENSHOT_HEIGHT':'540','TESSERAE_LOD':'0','TESSERAE_RESEARCH_NO_SHADOWS':'1','TESSERAE_RESEARCH_NO_MSAA':'1','TESSERAE_RESEARCH_FREEZE_ATLAS':'1'}}
for spec in sys.argv[1:]:
 name,_,rep=spec.partition(':');label=name+'-'+(rep or '1')
 env=dict(os.environ,BEVY_ASSET_ROOT=str(root),TESSERAE_MODS_DIR=str(root/'games/demo/dist'),TESSERAE_TRUST_MODS='1',TESSERAE_SCREENSHOT=str(out/(label+'.png')),TESSERAE_SCREENSHOT_OFFSCREEN='1',TESSERAE_SCREENSHOT_WIDTH='1920',TESSERAE_SCREENSHOT_HEIGHT='1080',TESSERAE_SCREENSHOT_FLY_S='12',TESSERAE_SCREENSHOT_FLY_AFTER_S='20',TESSERAE_SCREENSHOT_SETTLE_S='1',TESSERAE_RESEARCH_CSV=str(out/(label+'.csv')),RUST_LOG='info')
 env.update(variants[name]);meta={'label':label,'env':{k:v for k,v in env.items() if k.startswith('TESSERAE_') or k=='BEVY_ASSET_ROOT'},'start':time.time(),'uptime_before':subprocess.check_output(['uptime'],text=True).strip()}
 print('START',label,meta['uptime_before'],flush=True)
 with (out/(label+'.log')).open('w') as log:
  try:
   process=subprocess.Popen(['/usr/bin/caffeinate','-i',str(binary)],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT)
   if name in ('sample','metal'):
    try:process.wait(timeout=22)
    except subprocess.TimeoutExpired:pass
    cmd=['/usr/bin/sample',str(process.pid),'5','1','-file',str(out/(label+'.sample.txt'))] if name=='sample' else ['xcrun','xctrace','record','--template','Metal System Trace','--attach',str(process.pid),'--time-limit','5s','--output',str(out/(label+'.trace'))]
    with (out/(label+'.profiler.log')).open('w') as profiler: meta['profiler_exit']=subprocess.run(cmd,stdout=profiler,stderr=subprocess.STDOUT,timeout=45).returncode
   meta['exit']=process.wait(timeout=110)
  except subprocess.TimeoutExpired:
   process.terminate();process.wait(timeout=10);meta['exit']='timeout'
 meta['end']=time.time();meta['uptime_after']=subprocess.check_output(['uptime'],text=True).strip();(out/(label+'.json')).write_text(json.dumps(meta,indent=2))
 print('END',label,meta['exit'],round(meta['end']-meta['start'],2),flush=True)
 if meta['exit']!=0: print((out/(label+'.log')).read_text()[-5000:],flush=True);break
