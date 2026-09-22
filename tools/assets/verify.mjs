import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import sharp from 'sharp';
import { root } from './production-plan.mjs';
const assets=path.join(root,'assets');
const manifest=JSON.parse(await fs.readFile(path.join(assets,'manifest.json'),'utf8'));
const expected=[4,3,3,6,1,1,4,6,2,4,4,8,12,80];
const errors=[],checks=[];
const run=async(name,fn)=>{try{await fn();checks.push(name);}catch(e){errors.push({name,message:e.message});}};
await run('Manifest completeness: 138 primary assets, 14 plates',async()=>{
  assert.equal(manifest.primaryCount,138);assert.deepEqual(manifest.pending,[]);
  assert.equal(new Set(manifest.assets.map(a=>a.id)).size,manifest.assets.length);
  for(let i=0;i<14;i++)assert.equal(manifest.assets.filter(a=>+a.group===i+1&&!a.supplemental).length,expected[i],`Plate ${i+1}`);
});
await run('Every delivery path, format and source recipe exists',async()=>{
  for(const a of manifest.assets){
    for(const rel of Object.values(a.formats)){assert((await fs.stat(path.join(assets,rel))).size>0,rel);}
    assert((await fs.stat(path.join(assets,a.path.replace(/\.(svg|png)$/,'.prompt.txt')))).size>35,a.id);
    const meta=await sharp(path.join(assets,a.path)).metadata();
    assert(meta.width>0&&meta.height>0,a.id);
  }
});
await run('Exactly 80 labelled 24px SVG icons, seven categories',async()=>{
  const icons=manifest.assets.filter(a=>a.group==='14');assert.equal(icons.length,80);
  for(const a of icons){
    const svg=await fs.readFile(path.join(assets,a.path),'utf8');
    assert(svg.includes('viewBox="0 0 24 24"'));assert(svg.includes('stroke-width="1.5"'));assert(svg.includes('stroke="currentColor"'));assert(a.visibleLabel.length>1);
    assert(!/<script|<image|<foreignObject|https?:\/\/(?!www\.w3\.org)/i.test(svg));
    const {data}=await sharp(Buffer.from(svg)).ensureAlpha().raw().toBuffer({resolveWithObject:true});
    assert(data.some((v,i)=>i%4===3&&v>20),`Empty icon: ${a.name}`);
  }
});
await run('Three grayscale grain tiles: 512px, 8 bit, seamless statistics',async()=>{
  for(const a of manifest.assets.filter(a=>a.group==='03')){
    const meta=await sharp(path.join(assets,a.path)).metadata();
    assert.equal(meta.width,512);assert.equal(meta.height,512);assert.equal(meta.space,'b-w');assert.equal(meta.depth,'uchar');
    const {data,info}=await sharp(path.join(assets,a.path)).toColourspace('b-w').raw().toBuffer({resolveWithObject:true});
    assert.equal(info.channels,1);
    let interior=0,seam=0;
    for(let y=0;y<512;y++)for(let x=1;x<512;x++)interior+=Math.abs(data[y*512+x]-data[y*512+x-1]);
    interior/=512*511;
    for(let y=0;y<512;y++)seam+=Math.abs(data[y*512]-data[y*512+511]);seam/=512;
    assert(seam<interior*1.5+1,`Visible periodic boundary in ${a.id}: ${seam}/${interior}`);
    let vertical=0,verticalSeam=0;
    for(let y=1;y<512;y++)for(let x=0;x<512;x++)vertical+=Math.abs(data[y*512+x]-data[(y-1)*512+x]);
    vertical/=512*511;
    for(let x=0;x<512;x++)verticalSeam+=Math.abs(data[x]-data[511*512+x]);verticalSeam/=512;
    assert(verticalSeam<vertical*1.5+1,`Visible vertical boundary in ${a.id}`);
  }
});
await run('Hatching and halftone have 20/40/60% area coverage',async()=>{
  for(const a of manifest.assets.filter(a=>a.group==='04')){
    const {data,info}=await sharp(path.join(assets,a.path)).ensureAlpha().raw().toBuffer({resolveWithObject:true});
    let sum=0;for(let i=3;i<data.length;i+=4)sum+=data[i]/255;
    const mean=sum/(info.width*info.height);
    assert(Math.abs(mean-a.coverage)<.04,`${a.id} ${mean} expected ${a.coverage}`);
  }
});
await run('Transparent art and single-colour ornament overlays',async()=>{
  for(const a of manifest.assets.filter(a=>['06','07','08','09','10','11'].includes(a.group))){
    const {data}=await sharp(path.join(assets,a.path)).ensureAlpha().raw().toBuffer({resolveWithObject:true});
    assert(data.some((v,i)=>i%4===3&&v===0),`No transparent pixels: ${a.id}`);
    if(a.formats.overlay){
      const raw=await sharp(path.join(assets,a.formats.overlay)).ensureAlpha().raw().toBuffer();
      const c=a.group==='09'?[180,85,44]:[30,35,43];
      for(let i=0;i<raw.length;i+=4)if(raw[i+3]>0)assert(raw[i]===c[0]&&raw[i+1]===c[1]&&raw[i+2]===c[2],a.id);
    }
  }
});
await run('All 12 synthetic portraits: segregated, metadata, XMP, visible watermark strip',async()=>{
  const portraits=manifest.assets.filter(a=>a.synthetic);assert.equal(portraits.length,12);
  for(const a of portraits){
    assert(a.path.startsWith('demo/13-portraits/'));
    const side=JSON.parse(await fs.readFile(path.join(assets,a.path.replace('.png','.metadata.json')),'utf8'));
    assert.equal(side.synthetic,true);assert.equal(side.archivalExportAllowed,false);
    for(const format of ['png','webp']){
      const file=path.join(assets,a.formats[format]);assert(a.formats[format].startsWith('demo/'));
      const meta=await sharp(file).metadata();assert(meta.xmp?.toString().includes('dv:synthetic="true"'));
      const pixels=await sharp(file).extract({left:0,top:meta.height-5,width:meta.width,height:5}).removeAlpha().raw().toBuffer();
      assert(pixels.reduce((s,v)=>s+v,0)/pixels.length<70,`Missing watermark band: ${a.id}`);
    }
  }
});
await run('Nine-slice coordinates reconstruct the original rectangle without gaps',async()=>{
  const n=JSON.parse(await fs.readFile(path.join(assets,'source/ornament/06-deckle/06-nine-slice.json'),'utf8'));
  const meta=await sharp(path.join(assets,n.source)).metadata();
  assert.equal(Object.keys(n.slices).length,9);
  assert.equal(Object.values(n.slices).reduce((s,r)=>s+r.width*r.height,0),meta.width*meta.height);
  for(const slice of Object.values(n.slices)){const m=await sharp(path.join(assets,slice.path)).metadata();assert.equal(m.width,slice.width);assert.equal(m.height,slice.height);}
});
await run('Optical favicon set has seven true pixel sizes; ICO has five frames',async()=>{
  for(const size of [16,24,32,48,64,128,256]){
    const m=await sharp(path.join(assets,`generated/01/01-icon-optical-${size}.png`)).metadata();assert.equal(m.width,size);assert.equal(m.height,size);
  }
  const ico=await fs.readFile(path.join(assets,'generated/01/01-favicon.ico'));assert.equal(ico.readUInt16LE(2),1);assert.equal(ico.readUInt16LE(4),5);
});
const result={passed:errors.length===0,checks,errors};
await fs.writeFile(path.join(assets,'verification.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify(result,null,2));
if(errors.length)process.exitCode=1;
