// DreViGen design tokens — GENERATED, do not edit.
//
// Source: crates/drevigen-tokens/src/palette.rs
// Regenerate: cargo run -p drevigen-tokens -- generate

export type ThemeName = 'light' | 'dark' | 'contrast';

export type TokenName =
  | 'vellum'
  | 'vellum-deep'
  | 'well'
  | 'ink'
  | 'ink-soft'
  | 'ink-faint'
  | 'rule'
  | 'rule-soft'
  | 'sanguine'
  | 'sanguine-fill'
  | 'sage'
  | 'sepia'
  | 'verdigris'
  | 'indigo'
  | 'madder';

export const tokens: Record<ThemeName, Record<TokenName, string>> = {
  light: {
    vellum:        '#F7F3EB', // the page; every surface starts here
    vellum-deep:   '#EEE8DD', // recessed panels, plate wells
    well:          '#E5DED2', // the recess a photograph sits in
    ink:           '#0B131C', // primary text and engraved line; iron-gall, never pure black
    ink-soft:      '#48515B', // secondary text
    ink-faint:     '#6E757E', // captions and hairline labels
    rule:          '#7E7A71', // structural rules and borders; must be perceivable
    rule-soft:     '#D8D4CC', // the quietest divider
    sanguine:      '#9B3300', // the single accent as ink: focus, current person
    sanguine-fill: '#722000', // the accent as a surface, so its label can be read
    sage:          '#456442', // verified, sourced, confirmed
    sepia:         '#744F27', // inferred, approximate, uncertain
    verdigris:     '#1D6861', // places and geography
    indigo:        '#33487C', // the historical strata layer
    madder:        '#9C2527', // conflict, contradiction, open question
  },
  dark: {
    vellum:        '#18130E', // lamplit desk
    vellum-deep:   '#0E0A07', // recesses
    well:          '#211C17', // the recess a photograph sits in
    ink:           '#ECE7DE', // warm light text
    ink-soft:      '#BFBAB1', // secondary text
    ink-faint:     '#A29E96', // captions and hairline labels
    rule:          '#A19C96', // structural rules and borders; must be perceivable
    rule-soft:     '#2B2825', // the quietest divider
    sanguine:      '#FFC8A9', // the single accent as ink, lifted for a dark ground
    sanguine-fill: '#FFE3CF', // the accent as a surface, so its dark label can be read
    sage:          '#9DBB99', // verified, sourced, confirmed
    sepia:         '#D1AD84', // inferred, approximate, uncertain
    verdigris:     '#84C2BA', // places and geography
    indigo:        '#AAC0F2', // the historical strata layer
    madder:        '#FFA8A1', // conflict, contradiction, open question
  },
  contrast: {
    vellum:        '#FFFFFF', // pure page
    vellum-deep:   '#F5F3EF', // recessed panels
    well:          '#EAE7E2', // the recess a photograph sits in
    ink:           '#040609', // primary text
    ink-soft:      '#25292F', // secondary text
    ink-faint:     '#3E4349', // captions
    rule:          '#494E52', // borders, at 1.5 px minimum
    sanguine-fill: '#651A00', // the accent as a surface
    rule-soft:     '#717579', // the quietest divider
    sanguine:      '#882600', // the single accent
    sage:          '#2B4C28', // verified
    sepia:         '#5F3807', // inferred
    verdigris:     '#004E48', // places
    indigo:        '#1D3168', // historical strata
    madder:        '#83000D', // conflict
  },
};

export const ramps = {
  sanguine: {
    50: '#FFF3EF',
    100: '#FFE3D9',
    200: '#FFC6B3',
    300: '#FBA587',
    400: '#E88460',
    500: '#CD633C',
    600: '#AD4821',
    700: '#8A3009',
    800: '#651E00',
    900: '#421100',
    950: '#2B0800',
  },
  sage: {
    50: '#F1F8F0',
    100: '#E2EEE0',
    200: '#C8DCC6',
    300: '#ACC6A9',
    400: '#8EAD8A',
    500: '#70906C',
    600: '#577553',
    700: '#3F5A3C',
    800: '#2A4027',
    900: '#172916',
    950: '#0B190A',
  },
  sepia: {
    50: '#FDF4EB',
    100: '#F6E7D8',
    200: '#E9D0B9',
    300: '#D8B797',
    400: '#C19B75',
    500: '#A57D55',
    600: '#89633C',
    700: '#6B4A27',
    800: '#4E3315',
    900: '#331E07',
    950: '#211101',
  },
  verdigris: {
    50: '#ECF9F7',
    100: '#D9F0EC',
    200: '#B9DED9',
    300: '#96CAC3',
    400: '#72B1A9',
    500: '#4F948D',
    600: '#347972',
    700: '#1D5D57',
    800: '#09433E',
    900: '#002A27',
    950: '#001A17',
  },
  indigo: {
    50: '#F2F6FF',
    100: '#E0EAFF',
    200: '#C5D5F8',
    300: '#A9BDEC',
    400: '#8AA2D9',
    500: '#6C85BE',
    600: '#536AA0',
    700: '#3C507F',
    800: '#27375E',
    900: '#162240',
    950: '#0A142B',
  },
  madder: {
    50: '#FFF2F1',
    100: '#FFE2DF',
    200: '#FFC5BF',
    300: '#FFA098',
    400: '#EF7C75',
    500: '#D35A55',
    600: '#B33F3C',
    700: '#8F2727',
    800: '#6A1315',
    900: '#480307',
    950: '#300002',
  },
} as const;
