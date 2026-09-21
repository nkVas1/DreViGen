import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const brief = fs.readFileSync(path.join(root, 'docs/03-design/asset-brief.html'), 'utf8');
const decode = s => s.replaceAll('&lt;', '<').replaceAll('&gt;', '>').replaceAll('&amp;', '&').replaceAll('&quot;', '"');
const p = id => decode(brief.match(new RegExp(`<pre id="${id}">([\\s\\S]*?)</pre>`))[1]);
const common = p('p-preamble');
const no = p('p-negative');
const jobs = [];
function add(group, name, title, subject, width, height, folder, extra = '', photographic = false) {
  const prompt = photographic ? subject : `Use case: stylized-concept. Final production asset for DreViGen, Herbarium Vivum, plate ${group}.\n${common}\nSubject: ${subject}\n${extra}\nAvoid: ${no}.`;
  jobs.push({ id: `${group}-${name}`, group, name, title, prompt, width, height, folder, photographic });
}
add('01', 'engraved-emblem', 'Гравированный знак', 'A botanical cross-section of a seed.', 1024, 1024, 'source/identity/01-app-icon');
jobs[0].prompt = 'REPLACED_WITH_EXACT_ORIGINAL_PROMPT';
add('02', 'social-ground', 'Карточка репозитория', p('pr-2'), 1280, 640, 'source/identity/02-platform', 'Wide 2:1 composition. Match the three growth rings with single red centre seed emblem. Editorial museum quality, very subtle plate mark, rich natural laid vellum. The emblem occupies x=12–33% of the image, y=24–76%; the right two thirds stay completely blank. Opaque vellum background, no letters. Request a 2560 by 1280 image if available.');
add('05', 'foxing', 'Пятна времени', p('pr-5'), 1500, 1500, 'source/ornament/05-foxing', 'This is a material texture, not a drawing of an object: believable aged laid-paper surface. Delicate pale muted sepia foxing near outer 15%, clear centre. Square, 3000 by 3000 if available.');
add('06', 'deckle-plate', 'Рваный край и оттиск', p('pr-6'), 1600, 2000, 'source/ornament/06-deckle', 'One complete upright sheet, 4:5 portrait aspect, all four deckled edges visible within 4% margins. True transparent RGBA outside the sheet instead of a white backdrop. The sheet interior is warm cream #FBF7EE. A beautiful subtle plate impression, no printed black frame. 4000 by 5000 if available.');
const corners = [
  ['tl', 'Верхняя левая виньетка', 'upper-left', 'four alternating finely veined ovate beech-like leaves on an arching twig, a single closed terminal bud, twig flowing downward and right'],
  ['tr', 'Верхняя правая виньетка', 'upper-right', 'four slender lanceolate willow-like leaves spaced on an elegant curved twig, one closed pointed bud, twig flowing downward and left'],
  ['bl', 'Нижняя левая виньетка', 'lower-left', 'four small softly lobed oak-like leaves on an ascending delicate curved stem, one small closed terminal bud, twig flowing upward and right'],
  ['br', 'Нижняя правая виньетка', 'lower-right', 'four finely serrated birch-like leaves of varied size on a curved stem, one closed bud, twig flowing upward and left']
];
for (const [name, title, corner, foliage] of corners) add('07', `corner-${name}`, title,
  `A single restrained botanical corner ornament anchored in the ${corner} corner: ${foliage}. Botanically plausible, delicate fine copperplate hatching ONLY on the leaf undersides, thin confident organic contour, no flowers or wreaths.`, 800, 800, 'source/ornament/07-corners',
  'True transparent RGBA background. Ink #1E232B only, no colour or paper rectangle, no grey cast behind drawing. Entire twig visible, 8% breathing margin, composition travels diagonally inward and fills roughly 60% of canvas. Square 2000 pixels if available.');
const dividers = [
  ['laurel', 'Лавровый разделитель', 'four pairs of long narrow laurel leaves, two small curled tendrils'],
  ['oak', 'Дубовый разделитель', 'two pairs of small lobed oak leaves and understated acorn cups'],
  ['wheat', 'Колосовой разделитель', 'two slender wheat ears extending outward, fine parallel awns'],
  ['fern', 'Папоротниковый разделитель', 'two small arching fern fronds, disciplined fine pinnae'],
  ['linden', 'Липовый разделитель', 'two pairs of small heart-shaped linden leaves on graceful fine stalks'],
  ['vine', 'Усиковый разделитель', 'two slender leaf sprigs with long delicate spiral tendrils']
];
for (const [name, title, foliage] of dividers) add('08', `divider-${name}`, title,
  `${p('pr-8')} The specific motif in this divider is ${foliage}. Single small oval seed at exact centre.`, 1600, 200, 'source/ornament/08-dividers',
  'Ink #1E232B only. TRUE transparent RGBA, no cream or white canvas. The ornament itself is eight times wider than tall, centred with empty transparent space above and below, so it can be trimmed to an 8:1 strip. No border. Ultra-fine but clear and dark engraved outlines, absolutely symmetrical, 3200px wide if available.');
for (const [name, title, pressure] of [['even', 'Печать · ровный оттиск', 'Firm even impression, crisp dark lines'], ['uneven', 'Печать · мягкий прижим', 'Slightly broken impression toward upper right outer edge, incomplete tiny line segments where the stamp did not take']]) add('09', `seal-${name}`, title,
  `${p('pr-9')} ${pressure}. Exactly twelve small separate wheat ears, twelve evenly spaced around the ring.`, 800, 800, 'source/ornament/09-seal',
  'True transparent RGBA background. All visible linework a single terracotta ink #B4552C, no black, white or other ink, no shaded filled disc, no paper background, no text. Negative areas transparent. The seed has three concentric growth rings and a central dot. Square, 2000px if available.');
for (const [key, name, title] of [['a','empty-tree','Пока никого нет'], ['b','no-results','Ничего не найдено'], ['c','unsourced','Факт без источника'], ['d','offline','Нет связи']]) add('10', name, title, p(`pr-10${key}`), 960, 720, 'source/illustration/10-empty-states',
  'Final standalone illustration, landscape 4:3, 2400 by 1800 if available. True transparent RGBA around objects, no paper background rectangle. In a centred composition, the subject fills middle 55% width and 65% height; broad equal clear margins. Fine dark confident engraved detail that survives at 480px wide, sparse cross hatching, same line density as an 1860 botanical atlas. Use ONLY dark ink and, for a ribbon if present, muted terracotta. No modern UI, no labels.');
const onboards = [
 ['start', 'Начните с себя', 'A single pressed plant specimen mounted at the centre of an otherwise empty upright herbarium sheet, with one small blank paper label beneath it. The specimen is a fine slender stem with four small ovate leaves and a closed bud. One tiny terracotta archival mounting tab.'],
 ['parents', 'Добавьте родителей', 'An upright herbarium sheet carrying three pressed plant specimens: one small specimen below, two slightly larger parent specimens above, joined to the first by two fine hand-ruled lineage lines. One blank little specimen label beneath each plant. One tiny terracotta mounting tab on the bottom specimen.'],
 ['documents', 'Приложите документы', 'An upright herbarium sheet bearing one pressed plant specimen and a small blank label, beside which lie two small folded archive documents and a slender handled magnifying glass. All engraved in the same hand. One narrow terracotta ribbon around the archive documents.'],
 ['family', 'Позовите семью', 'Three upright herbarium sheets of identical design and size, each with a pressed plant specimen and one blank little label, lying slightly overlapped on a desk, as if three people had each brought their own. A single terracotta ribbon loosely ties them together.']
];
for (const [name,title,subject] of onboards) add('11', `onboarding-${name}`, title, subject, 960, 720, 'source/illustration/11-onboarding',
  'One image, landscape 4:3, 2400 by 1800 if available. Flat-on composition, NO perspective tilt or isometric. Paper sheet approximately 5:7 portrait proportions, softly cream filled paper, fine double hairline perimeter, specimen botanical engravings. True transparent background outside the objects. Whole group occupies centre 60% width and 75% height with clear margins, no cropping. Fine consistent blue-black engraved lines, one muted terracotta accent only. No numbers, words or writing anywhere. Patient calm nineteenth century scientific archive.');
const people = [
 ['1890s','woman-28','Женщина, 28 лет · 1890-е','a 28-year-old woman with a long oval face, dark centre-parted hair gathered at the nape, a modest high-neck cotton blouse with small cloth buttons'],
 ['1890s','man-42','Мужчина, 42 года · 1890-е','a 42-year-old man with broad cheekbones, a short full beard and moustache, a dark worn wool jacket over a simple collarless shirt'],
 ['1890s','woman-68','Женщина, 68 лет · 1890-е','a 68-year-old woman with a narrow angular face, deep natural wrinkles, a plain loosely tied headscarf and unadorned dark dress'],
 ['1890s','boy-12','Мальчик, 12 лет · 1890-е','a 12-year-old boy with short straight hair, an earnest round face, a plain buttoned school-age tunic'],
 ['1930s','woman-24','Женщина, 24 года · 1930-е','a 24-year-old woman with a softly square face, short practical dark bob, a simple blouse without ornament'],
 ['1930s','man-36','Мужчина, 36 лет · 1930-е','a 36-year-old clean-shaven man with a long face, side-parted hair and a work jacket over a plain shirt'],
 ['1930s','man-63','Мужчина, 63 года · 1930-е','a 63-year-old man with thinning grey hair, broad nose, short moustache and a modest worn wool waistcoat'],
 ['1930s','girl-10','Девочка, 10 лет · 1930-е','a 10-year-old girl with two simple braids, a broad round face and an everyday cotton dress with a small collar'],
 ['1960s','woman-31','Женщина, 31 год · 1960-е','a 31-year-old woman with softly waved shoulder-length hair, distinct high cheekbones, a simple collared everyday dress'],
 ['1960s','man-45','Мужчина, 45 лет · 1960-е','a 45-year-old man with a slightly receding hairline, a broad friendly weathered face, everyday shirt and plain cardigan'],
 ['1960s','woman-72','Женщина, 72 года · 1960-е','a 72-year-old woman with carefully combed silver hair, a full lined face, a simple blouse and knitted cardigan'],
 ['1960s','boy-15','Юноша, 15 лет · 1960-е','a 15-year-old boy with short slightly wavy hair, long narrow face and a plain collared shirt']
];
for (const [era,name,title,person] of people) {
  const technique = era==='1890s' ? '1890s glass plate negative, faded sepia brown silver-gelatin tone, soft single-source light from the left, subtle oval vignette' : era==='1930s' ? '1930s matte gelatin silver print, neutral warm monochrome, harder single-source light from the left, plain painted studio backdrop' : 'late 1950s to early 1960s black-and-white silver-gelatin print, neutral greys without sepia, softer contrast, soft left window light';
  add('13', `portrait-${era}-${name}`, title,
    `Use case: photorealistic-natural. One studio portrait of an entirely fictional synthetic person, not a real historical individual, for a clearly labelled genealogy app demo. ${person}, a provincial Russian family member. Photographic technique: ${technique}. Period-accurate modest everyday clothing, authentic fabric and skin, natural age details. Head and shoulders, upright 4:5 portrait, entire head and both shoulders within frame. Subject facing slightly off-camera, dignified neutral unsmiling expression. Consistent head scale: head from y=15% to 58%, shoulders at 72%. Visible subtle emulsion grain, slight edge vignette, at most one very fine scratch, faint natural print age. High quality restoration-grade scan of a believable historical studio photo. No modern glamor, beauty retouch, cinematic colour, fantasy costume, uniforms, medals, hat, text, captions, border, watermark or modern objects. Do not engrave or draw: this asset is a PHOTO, not illustration. 1600 by 2000 pixels if available.`,
    800, 1000, 'demo/13-portraits', '', true);
}
export { jobs };
if (process.argv[1] === fileURLToPath(import.meta.url)) console.log(JSON.stringify(jobs));
