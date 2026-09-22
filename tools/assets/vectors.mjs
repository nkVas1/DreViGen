// Hand-authored 24px interface glyphs and exact schematic plates.
// No third-party icon artwork. All geometry remains editable source.
export const groups = [
  ['navigation', 'Навигация', [
    ['home','Домой','M3 11 12 3 21 11 M5 10v11h5v-7h4v7h5V10'],
    ['search','Поиск','M16 16 21 21|c 10 10 7'],
    ['overview','Обзор','M3 3h18v18H3z M6 7h5v4H6z M9 10h9v8H9z'],
    ['legend','Легенда','M10 5h11 M10 12h11 M10 19h11 M3 3h4v4H3z M3 17h4v4H3z|c 5 12 2'],
    ['back','Назад','M20 12H4 M10 6l-6 6 6 6'],
    ['forward','Вперёд','M4 12h16 M14 6l6 6-6 6'],
    ['up','На уровень выше','M12 21V3 M6 9l6-6 6 6'],
    ['history','Недавнее','M3 4v6h6 M3 10a9 9 0 1 1 1 8 M12 7v5l4 2'],
    ['bookmark','Закладки','M6 3h12v18l-6-4-6 4z'],
    ['zoom-in','Приблизить','M16 16l5 5 M7 10h6 M10 7v6|c 10 10 7'],
    ['zoom-out','Отдалить','M16 16l5 5 M7 10h6|c 10 10 7'],
    ['fit','Показать всё','M3 8V3h5 M16 3h5v5 M21 16v5h-5 M8 21H3v-5 M8 8h8v8H8z']
  ]],
  ['family','Персона и семья', [
    ['person','Персона','M5 21v-3a7 7 0 0 1 14 0v3|c 12 6 3'],
    ['person-add','Добавить человека','M3 21v-3a6 6 0 0 1 10-4 M18 12v8 M14 16h8|c 9 6 3'],
    ['person-edit','Изменить данные','M3 21v-3a6 6 0 0 1 7-6 M12 19l-1 3 3-1 8-8-2-2z|c 9 6 3'],
    ['parents','Родители','M5 9v4h14V9 M12 13v5|c 5 5 3|c 19 5 3|c 12 21 2'],
    ['children','Дети','M12 7v5 M5 18v-6h14v6|c 12 4 3|c 5 21 2|c 19 21 2'],
    ['partners','Партнёры','M8 9h8 M4 14v7 M20 14v7 M2 21h6 M16 21h6|c 4 9 3|c 20 9 3'],
    ['siblings','Братья и сёстры','M12 3v6 M5 14V9h14v5 M2 22v-2a3 3 0 0 1 6 0v2 M16 22v-2a3 3 0 0 1 6 0v2|c 5 15 2|c 19 15 2'],
    ['ancestor','Предок','M12 3 20 11 12 19 4 11z M12 7l4 4-4 4-4-4z M12 19v3'],
    ['descendants','Потомки','M12 5v7 M5 18v-6h14v6|c 12 4 2|c 5 20 2|c 19 20 2'],
    ['living','Живущий человек','M12 3v9h9 M3 12a9 9 0 1 0 9-9|c 12 12 9'],
    ['deceased','Умерший человек','M12 3v18 M6 8h12'],
    ['birth','Рождение','M12 3v18 M3 12h18 M5.5 5.5l13 13 M5.5 18.5l13-13|c 12 12 3'],
    ['marriage','Брак','|c 8 12 6|c 16 12 6'],
    ['relationship','Родственная связь','M7 6h5v12h5|c 4 6 3|c 20 18 3'],
    ['pin','Закрепить','M8 3h8 M9 3v7l-3 4h12l-3-4V3 M12 14v8'],
    ['family-tree','Семейное древо','M12 7v5 M5 17v-5h14v5 M9 3h6v4H9z M2 17h6v4H2z M16 17h6v4h-6z']
  ]],
  ['sources','Источники', [
    ['source','Источник','M5 3h10l4 4v14H5z M15 3v5h4 M8 12h8 M8 16h6'],
    ['source-add','Добавить источник','M5 21V3h10l4 4v5 M15 3v5h4 M8 12h4 M16 15v7 M12.5 18.5h7'],
    ['archive','Архив','M3 4h18v5H3z M5 9v12h14V9 M9 13h6'],
    ['book','Книга','M12 5v16 M12 5C8 2 5 3 2 4v15c3-1 6-1 10 2 4-3 7-3 10-2V4c-3-1-6-2-10 1'],
    ['document','Документ','M5 3h10l4 4v14H5z M15 3v5h4 M8 11h8 M8 15h8 M8 18h5'],
    ['citation','Цитата','M9 5C5 5 3 9 3 13h6v7H3v-7 M21 5c-4 0-6 4-6 8h6v7h-6v-7'],
    ['verified','Подтверждено','M5 12l4 4 10-11|c 12 12 9'],
    ['uncertain','Предположительно','M9 8a3 3 0 1 1 5 2c-2 1-2 2-2 4 M12 18v.1|c 12 12 10'],
    ['conflict','Разночтение','M12 2 22 12 12 22 2 12z M12 7v6 M12 17v.1'],
    ['research','Исследование','M3 21 5 14 16 3l5 5-11 11z M5 14l5 5 M13 6l5 5 M3 21l5-2'],
    ['note','Заметка','M4 3h16v12l-6 6H4z M14 21v-6h6 M8 7h8 M8 11h6'],
    ['folder','Папка','M3 6h7l2 3h9v12H3z M3 6V3h7l2 3h9v3'],
    ['link','Связать','M10 14l4-4 M8 16l-1 1a4 4 0 0 1-6-6l5-5a4 4 0 0 1 6 0 M16 8l1-1a4 4 0 0 1 6 6l-5 5a4 4 0 0 1-6 0'],
    ['transcript','Расшифровка','M3 3h8v18H3z M5 7h4 M5 11h4 M15 5h6 M15 9h6 M15 13h6 M15 17h4']
  ]],
  ['media','Медиа', [
    ['image','Фотография','M3 3h18v18H3z M3 17l6-6 4 4 3-3 5 5|c 16 7 2'],
    ['image-add','Добавить фотографию','M11 21H3V3h18v8 M3 17l6-6 4 4 M18 14v8 M14 18h8|c 16 7 2'],
    ['scan','Сканировать','M3 8V3h5 M16 3h5v5 M21 16v5h-5 M8 21H3v-5 M3 12h18 M7 7h10v10H7z'],
    ['crop','Обрезать','M7 2v15h15 M2 7h15v15 M4 20l16-16'],
    ['rotate','Повернуть','M18 3v5h-5 M18 8a8 8 0 1 0 2 8 M8 10h6v6H8z'],
    ['download','Скачать','M12 3v12 M7 10l5 5 5-5 M3 16v5h18v-5'],
    ['upload','Загрузить','M12 16V3 M7 8l5-5 5 5 M3 16v5h18v-5'],
    ['print','Печать','M7 8V3h10v5 M7 17H3V8h18v9h-4 M7 14h10v7H7z M17 11h1'],
    ['original','Оригинал','M7 3h14v14h-4 M3 7h14v14H3z M6 17l4-5 4 5|c 7 11 1'],
    ['attachment','Вложение','M9 17 18 8a4 4 0 0 0-6-6L3 11a6 6 0 0 0 9 9l9-9 M7 14l8-8a1.5 1.5 0 0 1 2 2l-8 8a1.5 1.5 0 0 1-2-2']
  ]],
  ['sync','Синхронизация', [
    ['sync','Синхронизация','M3 4v6h6 M3 10a9 9 0 0 1 16-4 M21 20v-6h-6 M21 14a9 9 0 0 1-16 4'],
    ['offline','Нет связи','M3 3l18 18 M6 8a11 11 0 0 0-4 3 M10 6a12 12 0 0 1 12 5 M8 12l-2 2 M13 10a8 8 0 0 1 5 4 M9 17l3-2 3 2 M12 21v.1'],
    ['cloud','Облако','M7 19a5 5 0 1 1 0-10 6 6 0 0 1 12-1 5.5 5.5 0 0 1 0 11z'],
    ['local','На устройстве','M3 3h18v14H3z M8 21h8 M12 17v4'],
    ['contribution','Предложить изменение','M12 3v13 M8 12l4 4 4-4 M3 14v7h18v-7 M3 17h4l2 2h6l2-2h4'],
    ['review','Проверить изменения','M4 3h11v7 M4 3v18h8 M7 7h5 M7 11h3 M19 18l3 3|c 16 14 5'],
    ['accept','Принять','M4 12l5 5L20 5'],
    ['reject','Отклонить','M5 5l14 14 M19 5 5 19'],
    ['merge','Объединить','M6 20V4 M18 20v-4c0-6-12-3-12-10 M2 8l4-4 4 4|c 6 21 1|c 18 21 1'],
    ['compare','Сравнить','M12 3v18 M3 5h6v14H3z M15 5h6v14h-6z M5 9h2 M17 9h2 M5 13h2 M17 13h2'],
    ['backup','Резервная копия','M3 13v8h18v-8 M7 17h10 M12 15V3 M7 8l5-5 5 5'],
    ['restore','Восстановить','M3 13v8h18v-8 M7 17h10 M12 3v10 M7 8l5 5 5-5']
  ]],
  ['modes','Режимы', [
    ['atlas','Атлас','M5 7v5h14V7 M12 12v5 M2 3h6v4H2z M16 3h6v4h-6z M9 17h6v4H9z'],
    ['fan','Веер','M2 21a10 10 0 0 1 20 0z M6 21a6 6 0 0 1 12 0 M12 11v10 M5 14l7 7 7-7'],
    ['hourglass','Песочные часы','M4 3h16L4 21h16z M8 3v2 M16 3v2 M8 19v2 M16 19v2'],
    ['strata','Стратиграфия','M3 6h18 M3 12h18 M3 18h18 M6 3v12 M12 8v13 M18 3v13'],
    ['flows','Потоки','M3 3c9 0 9 3 18 3 M3 8c9 0 9 3 18 3 M3 11c9 0 9 5 18 5 M3 15c9 0 9 5 18 5 M3 18c9 0 9 2 18 2 M3 21h18 M3 3v5 M3 11v4 M3 18v3 M21 6v5 M21 16v5'],
    ['kinship','Родство','M5 19 12 5l7 14 M6 8l12 1 M4 20h2 M18 20h2|c 12 4 2|c 4 20 2|c 20 20 2'],
    ['genogram','Генограмма','M3 3h6v6H3z M9 6h6 M12 6v9 M6 18v-3h12v3 M3 18h6v4H3z|c 18 6 3|c 18 20 2'],
    ['geography','География','M3 5l6-2 6 2 6-2v16l-6 2-6-2-6 2z M9 3v16 M15 5v16 M4 12c4-4 11 5 16-1']
  ]],
  ['system','Система', [
    ['settings','Настройки','M3 6h5 M14 6h7 M3 12h11 M20 12h1 M3 18h2 M11 18h10|c 11 6 3|c 17 12 3|c 8 18 3'],
    ['help','Помощь','M9 8a3 3 0 1 1 5 2c-2 1-2 2-2 4 M12 18v.1|c 12 12 10'],
    ['accessibility','Доступность','M3 8l9 2 9-2 M12 10v5 M7 22l5-7 5 7|c 12 4 2'],
    ['theme','Свет и лампа','M15 3a9 9 0 1 0 6 14A10 10 0 0 1 15 3z'],
    ['language','Язык','M2 5h12 M8 2v3 M4 5c0 6 5 10 9 11 M12 5c0 6-5 10-9 11 M13 22l4-11 5 11 M15 18h5'],
    ['lock','Закрытый доступ','M5 10h14v11H5z M8 10V6a4 4 0 0 1 8 0v4 M12 14v3'],
    ['export','Экспорт','M11 3H3v18h18v-8 M10 14 21 3 M14 3h7v7'],
    ['import','Импорт','M10 3H3v18h18v-8 M21 3 10 14 M10 7v7h7']
  ]]
];
export function glyph(markup) {
  return markup.split('|').map((s, i) => i === 0 ? (s ? `<path d="${s}"/>` : '') : (() => {
    const [kind, x, y, r] = s.trim().split(' ');
    if (kind !== 'c') throw new Error('Unknown glyph primitive');
    return `<circle cx="${x}" cy="${y}" r="${r}"/>`;
  })()).join('');
}
export function iconSvg(name,label,body) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><title>${label}</title>${glyph(body)}</svg>`;
}
// The gated palette, from packages/tokens/dist/tokens.css. These are literals so that a
// rasteriser with no CSS context still produces the right colours; the class hooks below let
// the inlined SVG follow the theme, because CSS beats a presentation attribute.
//
// `currentColor` carries the ink: the root element sets `color`, which a rasteriser resolves
// and a stylesheet overrides.
const ink='currentColor', inkLiteral='#0B131C', accent='#9B3300', paper='#F7F3EB';
const path=d=>`<path d="${d}"/>`;
const line=(x1,y1,x2,y2,extra='')=>`<line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" ${extra}/>`;
const circle=(x,y,r=8,focus=false)=>`<circle cx="${x}" cy="${y}" r="${r}" ${focus?`class="dv-accent" fill="${accent}" stroke="${accent}"`:''}/>`;
const rect=(x,y,w,h,focus=false)=>`<rect x="${x}" y="${y}" width="${w}" height="${h}" ${focus?`class="dv-accent" stroke="${accent}" stroke-width="3"`:''}/>`;
const p=(r,a)=>[150+r*Math.cos(a),246-r*Math.sin(a)];
const pt=v=>v.map(x=>x.toFixed(3)).join(' ');
function arc(r,a,b){return `M${pt(p(r,a))} A${r} ${r} 0 0 0 ${pt(p(r,b))}`;}
export function modePlates(){
  const modes=[];
  let s=path('M48 88V113H116V88 M184 88V113H252V88 M82 113V152 M218 113V152 M82 178V207H218V178 M150 207V233');
  for(const x of [24,92,160,228])s+=rect(x,61,48,27);
  s+=rect(55,152,54,26)+rect(191,152,54,26)+rect(121,233,58,29,true);
  modes.push(['atlas','Атлас',s]);
  s='';
  for(const r of [42,80,119])s+=path(arc(r,0,Math.PI));
  s+=line(31,246,269,246);
  for(const a of [Math.PI/2])s+=line(...p(42,a),...p(119,a));
  for(const a of [Math.PI/4,3*Math.PI/4])s+=line(...p(80,a),...p(119,a));
  s+=circle(150,231,8,true);
  modes.push(['fan','Веер',s]);
  s=path('M45 58 92 103 M115 58 92 103 M185 58 208 103 M255 58 208 103 M92 103 150 150 208 103 M150 150 92 197 45 242 M92 197 115 242 M150 150 208 197 185 242 M208 197 255 242');
  for(const y of [58,242])for(const x of [45,115,185,255])s+=circle(x,y,6);
  for(const y of [103,197])for(const x of [92,208])s+=circle(x,y,7);
  s+=circle(150,150,10,true);modes.push(['hourglass','Песочные часы',s]);
  s='<defs><pattern id="strata-hatch" width="9" height="9" patternUnits="userSpaceOnUse"><path d="M-2 2 2-2 M0 9 9 0 M7 11 11 7" stroke="currentColor" stroke-width="0.65"/></pattern></defs><rect x="31" y="136" width="238" height="39" fill="url(#strata-hatch)" stroke="none" opacity=".24"/>';
  for(const y of [58,97,136,175,214,253])s+=line(31,y,269,y,'stroke-width="1" opacity=".45"');
  for(const [x,y1,y2,f] of [[66,43,154,false],[120,79,239,true],[180,112,213,false],[237,169,265,false]]){
    s+=`<g ${f?`class="dv-accent" stroke="${accent}"`:`stroke="${ink}"`} stroke-width="4">${line(x,y1,x,y2)}</g>`+line(x-6,y1,x+6,y1)+line(x-6,y2,x+6,y2);
  }modes.push(['strata','Стратиграфия',s]);
  // Three non-crossing flows, 24+18+12 = 24+30 units. Width preserved at endpoints.
  s='';
  const flows=[[63,24,78,false],[123,18,180,true],[183,12,198,false]];
  for(const [y,h,t,f] of flows)s+=`<path d="M42 ${y} C120 ${y} 180 ${t} 258 ${t} L258 ${t+h} C180 ${t+h} 120 ${y+h} 42 ${y+h}Z" ${f?`class="dv-accent" fill="${accent}" stroke="${accent}"`:`fill="${ink}" stroke="${ink}"`} fill-opacity="${f?'.2':'.08'}" stroke-width="1.5"/>`;
  for(const [y,h]of[[63,24],[123,18],[183,12]])s+=line(42,y,42,y+h,'stroke-width="4"');
  for(const [y,h]of[[78,24],[180,30]])s+=line(258,y,258,y+h,'stroke-width="4"');
  modes.push(['flows','Потоки',s]);
  const nodes=[[51,88],[147,51],[241,91],[99,159],[207,168],[58,246],[248,240]];
  s='<g stroke-opacity=".4">'+path('M51 88 99 159 58 246 M147 51 99 159 M147 51 207 168 248 240 M241 91 207 168')+'</g>';
  s+=`<path d="M58 246 99 159 147 51 207 168 248 240" class="dv-accent" stroke="${accent}" stroke-width="3"/>`;
  for(let i=0;i<nodes.length;i++)s+=circle(...nodes[i],8,i>=5);
  modes.push(['kinship','Родство',s]);
  s=rect(52,58,42,42)+circle(227,79,21)+path('M94 79H206 M150 79V166 M81 210V166H225V210')+rect(61,210,40,40,true)+circle(225,230,20);
  modes.push(['genogram','Генограмма',s]);
  s=rect(32,39,236,222)+'<g opacity=".38" stroke-width="1">';
  for(let n=0;n<6;n++)s+=path(`M32 ${65+n*31} C76 ${34+n*31} 113 ${89+n*31} 157 ${63+n*31} S225 ${74+n*31} 268 ${48+n*31}`);
  s+=path('M100 39C86 98 119 171 102 261 M194 39C211 120 180 191 195 261')+'</g>';
  for(const [x,y,f]of[[91,162,true],[184,103,false],[227,212,false]])s+=`<path d="M${x} ${y+23}C${x-12} ${y+10} ${x-17} ${y} ${x-17} ${y-7}C${x-17} ${y-30} ${x+17} ${y-30} ${x+17} ${y-7}C${x+17} ${y} ${x+12} ${y+10} ${x} ${y+23}Z" ${f?`class="dv-accent" fill="${accent}" stroke="${accent}"`:`fill="${paper}" stroke="${ink}"`}/><circle cx="${x}" cy="${y-7}" r="5" fill="${paper}" ${f?`class="dv-accent" stroke="${accent}"`:`stroke="${ink}"`}/>`;
  modes.push(['geography','География',s]);
  // `color` on the root is what `currentColor` resolves to when nothing sets it: a rasteriser
  // gets the gated ink, and a stylesheet on the inlined element overrides it for the theme.
  return modes.map(([name,title,body])=>({name,title,svg:`<svg xmlns="http://www.w3.org/2000/svg" width="600" height="600" viewBox="0 0 300 300" class="dv-plate" color="${inkLiteral}" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><title>${title}</title>${body}</svg>`}));
}
