// Wrap-around, seeded three-octave value noise. No random state outside this file.
export function random(seed) {
  let a=seed>>>0;
  return ()=>{a=(a+0x6D2B79F5)>>>0;let t=a;t=Math.imul(t^(t>>>15),t|1);t^=t+Math.imul(t^(t>>>7),t|61);return((t^(t>>>14))>>>0)/4294967296;};
}
const smooth=t=>t*t*(3-2*t);
function octave(size,cells,rng) {
  const grid=Float64Array.from({length:cells*cells},()=>rng());
  return(x,y)=>{
    const gx=x*cells/size,gy=y*cells/size,ix=Math.floor(gx),iy=Math.floor(gy),tx=smooth(gx-ix),ty=smooth(gy-iy);
    const at=(i,j)=>grid[((j+cells)%cells)*cells+(i+cells)%cells];
    return (at(ix,iy)*(1-tx)+at(ix+1,iy)*tx)*(1-ty)+(at(ix,iy+1)*(1-tx)+at(ix+1,iy+1)*tx)*ty;
  };
}
export function grain(variant,size=512) {
  const rng=random(0xD4E71000+{light:1,dark:2,print:3}[variant]);
  const fine=octave(size,256,rng),medium=octave(size,80,rng),broad=octave(size,8,rng);
  const pixels=Buffer.alloc(size*size);
  for(let y=0;y<size;y++)for(let x=0;x<size;x++){
    const n=.65*fine(x,y)+.25*medium(x,y)+.10*broad(x,y);
    // Integer cycles over the tile ensure the optional laid structure is seamless.
    const laid=variant==='print' ? 2*Math.cos(2*Math.PI*y*21/size)+Math.cos(2*Math.PI*x*426/size):0;
    const value=Math.round(198+93*(n-.5)+laid);
    pixels[y*size+x]=variant==='dark'?255-value:value;
  }
  return pixels;
}
export function hatch(kind,density) {
  const step=16, fill='#0B131C', ratio=density/100;
  let content;
  if(kind==='halftone'){
    const radius=Math.sqrt(ratio*step*step/Math.PI);
    content=`<circle cx="8" cy="8" r="${radius.toFixed(5)}" fill="${fill}"/>`;
  }else{
    // Cross-hatch coverage: 1-(1-singleCoverage)^2.
    const coverage=density===60?1-Math.sqrt(1-ratio):ratio;
    const width=coverage*step/Math.SQRT2;
    content=`<path d="M-8 8 8-8 M0 16 16 0 M8 24 24 8" fill="none" stroke="${fill}" stroke-width="${width.toFixed(5)}"/>`;
    if(density===60)content+=`<path d="M-8 8 8 24 M0 0 16 16 M8-8 24 8" fill="none" stroke="${fill}" stroke-width="${width.toFixed(5)}"/>`;
  }
  const id=`${kind}-${density}`;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><title>${kind} ${density}%</title><defs><pattern id="${id}" width="16" height="16" patternUnits="userSpaceOnUse">${content}</pattern></defs><rect width="64" height="64" fill="url(#${id})"/></svg>`;
}
