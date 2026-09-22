import fs from 'node:fs/promises';
import path from 'node:path';
import sharp from 'sharp';
const write=async(p,b)=>{await fs.mkdir(path.dirname(p),{recursive:true});await fs.writeFile(p,b);};
export async function finishing(assets,manifest){
  // Optical sizing: simplified topology, deliberately redrawn at each small pixel size.
  const entries=[];
  for(const size of [16,24,32,48,64,128,256]){
    const c=size/2,r=size*.39,stroke=Math.max(1,size*.046);
    const rings=size<=24?[r,r*.52]:[r,r*.68,r*.36];
    const svg=`<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}"><title>DreViGen</title><g fill="none" stroke="#0B131C" stroke-width="${stroke}">${rings.map(r=>`<circle cx="${c}" cy="${c}" r="${r}"/>`).join('')}</g><circle cx="${c}" cy="${c}" r="${size*.085}" fill="#9B3300"/></svg>`;
    const rel=`generated/01/01-icon-optical-${size}.png`;
    const png=await sharp(Buffer.from(svg)).png().toBuffer();
    await write(path.join(assets,rel),png);
    await write(path.join(assets,rel.replace('.png','.svg')),svg);
    if([16,32,48,64,256].includes(size))entries.push({size,png});
  }
  const header=Buffer.alloc(6+16*entries.length);header.writeUInt16LE(1,2);header.writeUInt16LE(entries.length,4);
  let offset=header.length;
  for(let i=0;i<entries.length;i++){
    const {size,png}=entries[i],p=6+i*16;
    header[p]=size===256?0:size;header[p+1]=size===256?0:size;
    header.writeUInt16LE(1,p+4);header.writeUInt16LE(32,p+6);header.writeUInt32LE(png.length,p+8);header.writeUInt32LE(offset,p+12);offset+=png.length;
  }
  await write(path.join(assets,'generated/01/01-favicon.ico'),Buffer.concat([header,...entries.map(e=>e.png)]));

  // The brief explicitly requires technical matte extraction/re-tinting at build time.
  // Preserve generated art masters. These are separate compositing derivatives.
  for(const item of manifest.filter(a=>['05','07','08','09'].includes(a.group))){
    const {data,info}=await sharp(path.join(assets,item.path)).ensureAlpha().raw().toBuffer({resolveWithObject:true});
    const colour=item.group==='09'?[180,85,44]:item.group==='05'?[138,106,68]:[30,35,43];
    const out=Buffer.alloc(data.length);
    for(let i=0;i<data.length;i+=4){
      let coverage;
      if(item.group==='05'){
        // Brown spot strength from the paper's blue-channel absorption; retain quiet pale areas.
        coverage=Math.max(0,Math.min(1,(224-data[i+2])/175));
      }else if(item.group==='09'){
        coverage=Math.max(0,Math.min(1,(247-data[i+1])/(247-85)));
      }else{
        coverage=Math.max(0,Math.min(1,(247-(data[i]+data[i+1]+data[i+2])/3)/(247-36)));
      }
      out[i]=colour[0];out[i+1]=colour[1];out[i+2]=colour[2];out[i+3]=Math.round(data[i+3]*coverage);
    }
    const rel=`generated/${item.group}/${item.id}-ink-overlay.png`;
    const overlay=await sharp(out,{raw:{width:info.width,height:info.height,channels:4}}).png().toBuffer();
    await write(path.join(assets,rel),overlay);
    item.formats.overlay=rel;

    // The shape is also written beside its master as a WebP alpha mask, because that is the
    // form the interface uses: `mask-image` plus `background-color` lets one ornament take the
    // theme's ink instead of being frozen in the grey it was drawn in. Committed rather than
    // left in generated/, so a clean clone can build the front end.
    const mask=`${item.path.replace(/@2x\.png$/,'')}-ink-mask.webp`;
    await write(path.join(assets,mask),await sharp(overlay).webp({quality:90,effort:6,alphaQuality:100}).toBuffer());
    item.formats.inkMask=mask;
    await write(path.join(assets,rel.replace('.png','.prompt.txt')),`Method: deterministic matte extraction and single-ink re-tint from ${item.path}.\nSource: tools/assets/finishing.mjs. Model: none. Seed: none.\nOriginal image prompt: see parent PNG sidecar. Separate derivative; native master alpha remains untouched.\n`);
  }
}
