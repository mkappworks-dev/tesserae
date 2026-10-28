import csv,json,re,math,sys
from pathlib import Path
out=Path(sys.argv[1])
def quant(a,p):
 a=sorted(a);return a[max(0,math.ceil(len(a)*p)-1)] if a else None
def stats(a):
 return {'n':len(a),'p50':quant(a,.5),'p95':quant(a,.95),'p99':quant(a,.99),'max':max(a) if a else None}
all=[]
for p in sorted(out.glob('*.csv')):
 if p.name.endswith(('.spans.csv','.diagnostics.csv')):continue
 rows=list(csv.DictReader(p.open()));log=p.with_suffix('.log').read_text();match=re.search(r'capture shown_s=\S+ fill_s=\S+.*',log)
 windows={'warm_still':(16,20),'flight':(22,32)};result={'run':p.stem,'log':match.group(0) if match else None,'windows':{}}
 for label,(lo,hi) in windows.items():
  selected=[r for r in rows if lo<float(r['elapsed_s'])<=hi]
  v={k:stats([float(r[k]) for r in selected]) for k in ('frame_ms','tick_ms','upload_ms','sort_ms')}
  v['combined_ms']=stats([float(r['tick_ms'])+float(r['upload_ms'])+float(r['sort_ms']) for r in selected]);v['frames_above_6_94_ms']=sum(float(r['frame_ms'])>1000/144 for r in selected);v['frames_above_10_ms']=sum(float(r['frame_ms'])>10 for r in selected)
  sp=p.with_suffix('.spans.csv')
  if sp.exists():
   spans=list(csv.DictReader(sp.open()));assert len(spans)==len(rows)
   v['main_schedule_wall_ms']=stats([float(span['main_schedule_wall_ms']) for row,span in zip(rows,spans) if lo<float(row['elapsed_s'])<=hi])
  result['windows'][label]=v
 all.append(result)
(out/'summary.json').write_text(json.dumps(all,indent=2))
for r in all:
 v=r['windows']['flight'];f=v['frame_ms'];m=v.get('main_schedule_wall_ms',{})
 print(r['run'], 'n',f['n'],'frame',*[round(f[k],3) if f[k] else None for k in ('p50','p95','p99','max')], 'main_p50',round(m.get('p50') or 0,3),'cost_p99',round(v['combined_ms']['p99'] or 0,3))
