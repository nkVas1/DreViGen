import fs from 'node:fs/promises';
import path from 'node:path';
import crypto from 'node:crypto';
import sharp from 'sharp';
import { root, jobs } from './production-plan.mjs';
import { groups, glyph, iconSvg, modePlates } from './vectors.mjs';
import { grain, hatch } from '../textures/generate-grain.mjs';
import { makeCatalog } from './catalog.mjs';
import { finishing } from './finishing.mjs';

const assets=path.join(root,'assets');
const scratch=path.join(root,'.design-scratch/asset-generation');
const manifest=[];
const exists=async p=>{try{await fs.access(p);return true;}catch{return false;}};
const write=async(p,data)=>{await fs.mkdir(path.dirname(p),{recursive:true});await fs.writeFile(p,data);};
const esc=s=>s.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('"','&quot;');
const sha=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
const record=(group,name,title,relative,extra={})=>{
  const entry={id:`${group}-${name}`,group,name,title,path:relative,...extra};manifest.push(entry);return entry;
};
async function sidecar(file,text){await write(file.replace(/\.(png|svg|avif|webp)$/i,'.prompt.txt'),text);}
async function vector(group,name,title,folder,svg,size=600,extra={}){
  const rel=`${folder}/${group}-${name}.svg`,dst=path.join(assets,rel);
  await write(dst,svg);
  await sidecar(dst,`Method: deterministic SVG authored for DreViGen.\nModel: none. Seed: not applicable.\nSource: tools/assets/vectors.mjs or existing assets/identity/ generator.\nSpecification: ${title}. ${extra.spec??''}\nPalette: the gated tokens only — currentColor for ink, #9B3300 for the single accent.\n`);
  const png=rel.replace('.svg','@2x.png');
  await sharp(Buffer.from(svg)).resize(size,size).png({compressionLevel:9}).toFile(path.join(assets,png));
  await sidecar(path.join(assets,png),`Method: automated raster derivative of ${rel}.\nModel: none. Seed: not applicable.\nSource: tools/assets/build.mjs; rendered at ${size} × ${size}.\n`);
  return record(group,name,title,rel,{method:'vector',formats:{svg:rel,png},width:size,height:size,...extra});
}

// Plates 01–02: the application mark is one code-native system, including at 16px.
// Preserve the current Rust generator and its production files; provide numbered exports.
const identity='source/identity/01-app-icon';
const platform='source/identity/02-platform';
const originals={};
for(const n of ['mark','mark-dark','adaptive-foreground','adaptive-background','adaptive-monochrome'])originals[n]=await fs.readFile(path.join(assets,`identity/${n}.svg`),'utf8');
const withoutGround=s=>s.replace(/<rect\b[^>]*width="1024"[^>]*\/>/,'');
for(const [name,title,svg]of[
  ['icon-light','Знак · дневной свет',withoutGround(originals.mark)],
  ['icon-dark','Знак · лампа',withoutGround(originals['mark-dark'])],
  ['icon-monochrome','Знак · монохром',withoutGround(originals.mark).replaceAll('#9B3300','#0B131C').replaceAll('opacity="0.55"','opacity="1"')],
  ['icon-vellum','Знак · на вельме',originals.mark]
])await vector('01',name,title,identity,svg,1024,{spec:'Three growth rings; original Rust mark geometry. Transparent except the vellum edition.'});
for(const [n,title]of[['adaptive-foreground','Android · передний слой'],['adaptive-background','Android · фон']]){
  const item=await vector('02',n,title,platform,originals[n],432,{safeZone:n==='adaptive-foreground'?0.66:null});
  item.width=432;item.height=432;
}

// A social plate with the exact same identity. Text is a live layer, never image-generated.
const markInner=withoutGround(originals.mark).replace(/<svg[^>]*>|<\/svg>/g,'');
const social=`<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="640" viewBox="0 0 1280 640"><title>DreViGen social card background</title><defs><filter id="paper"><feTurbulence type="fractalNoise" baseFrequency=".72" numOctaves="3" stitchTiles="stitch"/><feColorMatrix type="saturate" values="0"/></filter></defs><rect width="1280" height="640" fill="#F7F3EB"/><rect width="1280" height="640" filter="url(#paper)" opacity=".035"/><rect x="27" y="27" width="1226" height="586" fill="none" stroke="#7E7A71" stroke-opacity=".28"/><rect x="29" y="29" width="1222" height="582" fill="none" stroke="#fff" stroke-opacity=".5"/><g transform="translate(100 169) scale(.294921875)">${markInner}</g></svg>`;
const socialRel=`${platform}/02-social-card.svg`;
await write(path.join(assets,socialRel),social);
await sharp(Buffer.from(social)).png().toFile(path.join(assets,`${platform}/02-social-card@1x.png`));
await sidecar(path.join(assets,socialRel),'Method: SVG composition of the existing mark, procedural grain and plate impression.\nModel: none. Seed: SVG feTurbulence default 0.\nSpecification: 1280×640, mark in left third; right two thirds blank for live typography.\n');
await sidecar(path.join(assets,`${platform}/02-social-card@1x.png`),'Method: deterministic PNG export of 02-social-card.svg. Model: none. Seed: 0.\n');
record('02','social-card','Карточка репозитория',socialRel,{method:'vector',width:1280,height:640,formats:{svg:socialRel,png:`${platform}/02-social-card@1x.png`}});

// Reproducible seamless material tiles.
for(const variant of ['light','dark','print']){
  const rel=`source/textures/03-paper-grain/03-grain-${variant}@1x.png`;
  await fs.mkdir(path.dirname(path.join(assets,rel)),{recursive:true});
  await sharp(grain(variant),{raw:{width:512,height:512,channels:1}}).toColourspace('b-w').png().toFile(path.join(assets,rel));
  await sidecar(path.join(assets,rel),`Method: seeded periodic value noise; 256, 80, 8 cells per 512px tile; weights .65/.25/.10.\nModel: none. Seed: ${0xD4E71000+{light:1,dark:2,print:3}[variant]}.\nSource: tools/textures/generate-grain.mjs\nVariant: ${variant}. 8-bit grayscale; multiply .04–.07 (dark: screen .035). Never animate.\n`);
  record('03',`grain-${variant}`,{light:'Бумага · дневной свет',dark:'Бумага · лампа',print:'Бумага · печать'}[variant],rel,{method:'procedural',width:512,height:512,seamless:true,formats:{png:rel}});
}
for(const kind of ['hatch','halftone'])for(const density of [20,40,60]){
  const svg=hatch(kind,density),folder='source/textures/04-hatching';
  await vector('04',`${kind}-${density}`,`${kind==='hatch'?'Штриховка':'Растр'} · ${density}%`,folder,svg,128,{method:'procedural',seamless:true,coverage:density/100,spec:'Analytic density, seamless 16px period, 45° hatching; 60% crossed 45°/135°.'});
}

// Image generation receipts contain exact prompts and unmodified native master locations.
// Copy/resize/export only; do not invent extra pixels to satisfy a nominal 4× request.
const pending=[];
for(const job of jobs){
  if(job.id==='02-social-ground')continue; // Native identity is authoritative; keep the generated study outside delivery.
  const rel=`${job.folder}/${job.id}@2x.png`,dst=path.join(assets,rel);
  const receiptFile=path.join(scratch,`${job.id}${await exists(path.join(scratch,`${job.id}-r2.json`))?'-r2':''}.json`);
  const receipt=await exists(receiptFile)?JSON.parse(await fs.readFile(receiptFile,'utf8')):null;
  const original=receipt?.original;
  if(!original||!await exists(original)||process.argv.includes('--from-delivery')){
    if(!await exists(dst)){pending.push(job.id);continue;}
    const meta=await sharp(dst).metadata();
    const provenance=JSON.parse(await fs.readFile(dst.replace('.png','.provenance.json'),'utf8'));
    record(job.group,job.name,job.title,rel,{method:'image_gen',width:meta.width,height:meta.height,alpha:meta.hasAlpha,synthetic:job.group==='13',supplemental:job.group==='01',formats:{png:rel},nativeSize:[provenance.native.width,provenance.native.height],theme:job.group==='06'?'print-only':'light'});
    continue;
  }
  const native=await sharp(original).metadata();
  await fs.mkdir(path.dirname(dst),{recursive:true});
  const size={width:Math.min(job.width,native.width),height:Math.min(job.height,native.height)};
  let pipeline=sharp(original);
  if(job.group==='08')pipeline=pipeline.trim({threshold:10});
  pipeline=pipeline.resize(size.width,size.height,{fit:'contain',withoutEnlargement:true,background:{r:0,g:0,b:0,alpha:0}});
  let buffer=await pipeline.png().toBuffer();
  if(job.group==='13'){
    const label=Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="${size.width}" height="${size.height}"><rect x="0" y="${size.height-51}" width="${size.width}" height="51" fill="#0B131C" fill-opacity=".94"/><text x="22" y="${size.height-18}" font-family="Arial, sans-serif" font-size="19" letter-spacing="1" fill="#F7F3EB">ДЕМО · СИНТЕТИЧЕСКИЙ ПОРТРЕТ</text></svg>`);
    const xmp=`<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:dv="https://drevigen.org/ns/demo/1.0/" xmlns:Iptc4xmpExt="http://iptc.org/std/Iptc4xmpExt/2008-02-29/" dv:synthetic="true" dv:usage="demo-only" Iptc4xmpExt:DigitalSourceType="http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia"/></rdf:RDF></x:xmpmeta>`;
    buffer=await sharp(buffer).composite([{input:label}]).withXmp(xmp).png().toBuffer();
    await write(dst.replace('.png','.metadata.json'),JSON.stringify({synthetic:true,usage:'demo-only',archivalExportAllowed:false,id:job.id,description:job.title,watermark:'burned-in, bottom strip',model:receipt.model,seed:null},null,2)+'\n');
  }
  await write(dst,buffer);
  const meta=await sharp(buffer).metadata();
  const exact=`Method: built-in image_gen.\nModel: ${receipt.model}.\nSeed: not exposed by the tool (not fabricated).\nDate: ${receipt.createdAt}.\nNative master: ${native.width} × ${native.height}.\nDelivered: ${meta.width} × ${meta.height}.\nProcessing: automated contain/downsample via tools/assets/build.mjs${job.group==='08'?'; trim transparent margins':''}${job.group==='13'?'; visible synthetic watermark and embedded XMP':''}. Native alpha preserved.\n\nEXACT GENERATION PROMPT\n${receipt.prompt}\n`;
  await sidecar(dst,exact+(receipt.previousPrompt?`\nINITIAL GENERATION PROMPT\n${receipt.previousPrompt}\n\nREFERENCE ASSETS\n${receipt.referenceIds.join('\n')}\n`:''));
  const sourceRef=dst.replace('.png','.provenance.json');
  await write(sourceRef,JSON.stringify({id:job.id,tool:'image_gen',model:receipt.model,seed:null,createdAt:receipt.createdAt,native:{width:native.width,height:native.height,sha256:sha(await fs.readFile(original))},output:{width:meta.width,height:meta.height,sha256:sha(buffer)},synthetic:job.group==='13',fullResolutionRequestHonoured:native.width>=job.width*2&&native.height>=job.height*2},null,2)+'\n');
  record(job.group,job.name,job.title,rel,{method:'image_gen',width:meta.width,height:meta.height,alpha:meta.hasAlpha,synthetic:job.group==='13',supplemental:job.group==='01',formats:{png:rel},nativeSize:[native.width,native.height],theme:job.group==='06'?'print-only':'light'});
}

// Accurate, infinitely scalable mode diagrams.
for(const {name,title,svg}of modePlates())await vector('12',`mode-${name}`,title,'source/modes/12-display-modes',svg,600);
const iconManifest=[];
let groupIndex=0;
const symbols=[];
for(const [category,categoryTitle,icons]of groups){
  groupIndex++;
  for(const [name,label,body]of icons){
    const id=`${String(groupIndex).padStart(2,'0')}-${name}`;
    const relative=`14-interface/${String(groupIndex).padStart(2,'0')}-${category}/${id}.svg`;
    const full=path.join(root,'packages/ui/icons',relative);
    await write(full,iconSvg(name,label,body));
    await sidecar(full,`Method: original SVG geometry on a 24px grid.\nModel: none. Seed: not applicable.\nSource: tools/assets/vectors.mjs\nStroke: 1.5px, round caps and joins; currentColor.\nVisible label (required): ${label}.\n`);
    const item={name,label,category,categoryTitle,path:relative};iconManifest.push(item);
    symbols.push(`<symbol id="dv-${name}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">${glyph(body)}</symbol>`);
    record('14',name,label,`../packages/ui/icons/${relative}`,{method:'vector',category,width:24,height:24,visibleLabel:label,formats:{svg:`../packages/ui/icons/${relative}`}});
  }
}
await write(path.join(root,'packages/ui/icons/14-interface/sprite.svg'),`<svg xmlns="http://www.w3.org/2000/svg">${symbols.join('')}</svg>`);
await write(path.join(root,'packages/ui/icons/14-interface/manifest.json'),JSON.stringify(iconManifest,null,2)+'\n');
await write(path.join(root,'packages/ui/icons/14-interface/labels.ts'),`// Generated from tools/assets/vectors.mjs. Keep the visible label beside each icon.\nexport const iconLabels = ${JSON.stringify(Object.fromEntries(iconManifest.map(i=>[i.name,i.label])),null,2)} as const;\nexport type IconName = keyof typeof iconLabels;\n`);

// Portable app-size derivatives: rebuilt from committed, numbered source files.
for(const item of manifest){
  if(item.group==='14')continue;
  const png=item.formats.png;
  if(!png)continue;
  const dir=item.synthetic?'demo/13-portraits/derivatives':`generated/${item.group}`;
  const derivative=`${dir}/${item.id}@1x.webp`;
  await fs.mkdir(path.dirname(path.join(assets,derivative)),{recursive:true});
  let img=sharp(path.join(assets,png)).resize(Math.max(1,Math.round(item.width/2)),Math.max(1,Math.round(item.height/2)),{fit:'inside',withoutEnlargement:true});
  if(item.synthetic)img=img.keepXmp();
  await img.webp({quality:90,alphaQuality:100,effort:5}).toFile(path.join(assets,derivative));
  item.formats.webp=derivative;
  if(item.synthetic){
    await sidecar(path.join(assets,derivative),`Method: WebP derivative of ${png}.\nModel and exact prompt: see parent PNG prompt sidecar.\nSynthetic: true. Usage: demo-only. Watermark preserved; embedded XMP preserved.\n`);
    await write(path.join(assets,derivative.replace('.webp','.metadata.json')),JSON.stringify({synthetic:true,usage:'demo-only',archivalExportAllowed:false,parent:png},null,2)+'\n');
  }
}
// Nine-slice coordinates refer to the delivered deckle master, not a stretched photograph.
const deckle=manifest.find(i=>i.id==='06-deckle-plate');
if(deckle){
  const w=deckle.width,h=deckle.height,left=Math.round(w*.18),top=Math.round(h*.15),right=left,bottom=top;
  const specs={topLeft:[0,0,left,top],top:[left,0,w-left-right,top],topRight:[w-right,0,right,top],left:[0,top,left,h-top-bottom],centre:[left,top,w-left-right,h-top-bottom],right:[w-right,top,right,h-top-bottom],bottomLeft:[0,h-bottom,left,bottom],bottom:[left,h-bottom,w-left-right,bottom],bottomRight:[w-right,h-bottom,right,bottom]};
  const slices={};
  for(const [key,[x,y,width,height]]of Object.entries(specs)){
    const rel=`generated/06/nine-slice/06-deckle-${key}.png`;
    await fs.mkdir(path.dirname(path.join(assets,rel)),{recursive:true});
    await sharp(path.join(assets,deckle.path)).extract({left:x,top:y,width,height}).png().toFile(path.join(assets,rel));
    slices[key]={path:rel,x,y,width,height};
  }
  await write(path.join(assets,'source/ornament/06-deckle/06-nine-slice.json'),JSON.stringify({source:deckle.path,insets:{top,right,bottom,left},edgeMode:'stretch',centreMode:'stretch',usage:'Print only. Preserve corner dimensions. For large-format print use native master, never silently upscale.',slices},null,2)+'\n');
}
await finishing(assets,manifest);
const output={schemaVersion:1,collection:'Herbarium Vivum',date:'2026-09-22',brief:'docs/03-design/asset-brief.html',primaryCount:manifest.filter(i=>!i.supplemental).length,assets:manifest,pending,notes:['Image generator chose its native output dimensions; no fabricated 4× masters. See each provenance sidecar.','Original masters remain outside the repository; source contains optimized production sizes.','Synthetic portrait pixels, XMP and sidecar are marked. A future archive importer must reject demo-only assets.','App identity uses its existing Rust geometry and resolved palette; engraved emblem is a supplemental print asset.']};
await write(path.join(assets,'manifest.json'),JSON.stringify(output,null,2)+'\n');
await write(path.join(assets,'catalog.html'),makeCatalog(output,iconManifest));
console.log(JSON.stringify({delivered:manifest.length,primary:output.primaryCount,pending,groups:Object.fromEntries(Array.from({length:14},(_,i)=>{const g=String(i+1).padStart(2,'0');return[g,manifest.filter(a=>a.group===g).length];}))},null,2));
