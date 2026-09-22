/**
 * The application shell, shared by the Tauri build and the web build.
 *
 * What is here is deliberately small and deliberately real. The theme and text-scale controls
 * are not scaffolding — they are permanent product requirements from
 * `docs/01-research/elder-ux-and-accessibility.md`, and building them first means every screen
 * that follows is built inside them rather than retrofitted to them.
 *
 * What is *not* here is anything that pretends to work. There is no "create a tree" button
 * leading nowhere: the data layer lands in Phase 1, and the screen says so in plain words
 * rather than offering an action that would disappoint.
 */

import { useCallback, useEffect, useState } from 'react';

import { IconLabel, type IconName } from '@drevigen/ui';
import '@drevigen/ui/ui.css';

import './app.css';
import seedling from '../../../assets/source/illustration/10-empty-states/10-empty-tree@2x.png';
import {
  TEXT_SCALES,
  type Platform,
  type TextScale,
  type ThemeChoice,
  applyTextScale,
  applyTheme,
  detectPlatform,
  readTextScale,
  readTheme,
} from './platform';

/** Labels in Russian, the primary locale. ICU catalogues arrive with the i18n package. */
const THEME_LABELS: Record<ThemeChoice, string> = {
  system: 'Как в системе',
  light: 'День',
  dark: 'Лампа',
  contrast: 'Контраст',
};

const THEME_ORDER: ThemeChoice[] = ['system', 'light', 'dark', 'contrast'];

/**
 * A group of mutually exclusive buttons with a visible text label.
 *
 * Deliberately not a `<select>`: a segmented control shows every option at once, which removes
 * the memory step of opening a menu to see what is available. That is the COGA guidance, and
 * it is why the label is text rather than an icon.
 */
function Choice<T extends string | number>({
  label,
  options,
  value,
  format,
  onChange,
}: {
  label: string;
  options: readonly T[];
  value: T;
  format: (option: T) => string;
  onChange: (option: T) => void;
}) {
  return (
    <div className="dv-field">
      <span className="dv-label" id={`label-${label}`}>
        {label}
      </span>
      <div className="dv-choice" role="group" aria-labelledby={`label-${label}`}>
        {options.map((option) => (
          <button
            key={String(option)}
            type="button"
            aria-pressed={option === value}
            onClick={() => onChange(option)}
          >
            {format(option)}
          </button>
        ))}
      </div>
    </div>
  );
}

/**
 * The specimen: plate 10 of the collection, an engraving with a transparent ground.
 *
 * Not decoration. The screen has one subject and a great deal of space around it, which is the
 * composition of a botanical plate and the reason the art direction chose one.
 */
function Specimen() {
  return (
    <figure className="dv-specimen-plate">
      <img
        src={seedling}
        width={960}
        height={720}
        alt="Проросток с двумя семядолями и корневой системой — гравюра"
        decoding="async"
      />
    </figure>
  );
}

/** The semantic colours, shown so the palette can be verified on the device it runs on. */
/**
 * A sample of the icon set, shown for the same reason the colour swatches are: a glyph that has
 * to read at 24 px on a phone is verified on a phone. One from each of the seven categories.
 */
const SPECIMEN_ICONS: IconName[] = [
  'family-tree',
  'person',
  'source',
  'image',
  'sync',
  'strata',
  'accessibility',
];

const SEMANTIC = [
  ['sanguine', 'акцент'],
  ['sage', 'подтверждено'],
  ['sepia', 'приблизительно'],
  ['verdigris', 'место'],
  ['indigo', 'эпоха'],
  ['madder', 'разночтение'],
] as const;

export function App() {
  const [platform] = useState<Platform>(detectPlatform);
  const [theme, setTheme] = useState<ThemeChoice>(readTheme);
  const [scale, setScale] = useState<TextScale>(readTextScale);

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  useEffect(() => {
    applyTextScale(scale);
  }, [scale]);

  const formatScale = useCallback((value: TextScale) => `${value} %`, []);

  return (
    <div className="dv-shell">
      <header className="dv-chrome">
        <span className="dv-wordmark">
          Дре<span>Vi</span>Gen
        </span>
        <div className="dv-controls">
          <Choice
            label="Оформление"
            options={THEME_ORDER}
            value={theme}
            format={(option) => THEME_LABELS[option]}
            onChange={setTheme}
          />
          <Choice
            label="Размер текста"
            options={TEXT_SCALES}
            value={scale}
            format={formatScale}
            onChange={setScale}
          />
        </div>
      </header>

      <main className="dv-main">
        <article className="dv-plate">
          <Specimen />
          <h1 className="dv-title">Начните с себя</h1>
          <p className="dv-lede">
            Семейный архив, по которому можно путешествовать как по карте, хранить как реликвию
            и исследовать как учёный.
          </p>

          <hr className="dv-divider-ornament" />

          <section className="dv-state">
            <p>
              Это <strong>основание</strong> — фаза&nbsp;0. Работают оформление, размер текста,
              шрифты и палитра; редактор древа появится в фазе&nbsp;1. Кнопки, которая ничего не
              делает, здесь нет намеренно.
            </p>
            <dl>
              <dt>Оболочка</dt>
              <dd>{platform.shell === 'tauri' ? 'Tauri' : 'браузер'}</dd>
              <dt>Установлено</dt>
              <dd>{platform.installed ? 'да' : 'нет'}</dd>
              <dt>Ввод</dt>
              <dd>{platform.touch ? 'касание' : 'указатель'}</dd>
              <dt>Меньше движения</dt>
              <dd>{platform.reducedMotion ? 'да' : 'нет'}</dd>
              <dt>Шифр</dt>
              <dd>
                <code>ГААО ф.29 оп.1 д.204 л.17об</code>
              </dd>
              <dt>Годы</dt>
              <dd>1771 · 1804 · 1918 · 1943</dd>
            </dl>

            <div className="dv-specimen">
              {SEMANTIC.map(([token, meaning]) => (
                <figure key={token} style={{ margin: 0 }}>
                  <div
                    className="dv-swatch"
                    style={{ background: `var(--dv-${token})` }}
                    role="img"
                    aria-label={`${token}: ${meaning}`}
                  />
                  <figcaption
                    style={{
                      fontSize: '0.72rem',
                      marginTop: 'var(--dv-space-1)',
                      color: 'var(--dv-ink-faint)',
                    }}
                  >
                    {meaning}
                  </figcaption>
                </figure>
              ))}
            </div>

            <div className="dv-icon-specimen">
              {SPECIMEN_ICONS.map((name) => (
                <IconLabel key={name} name={name} size="small" />
              ))}
            </div>
          </section>
        </article>
      </main>
    </div>
  );
}

export { detectPlatform } from './platform';
export type { Platform, TextScale, ThemeChoice } from './platform';
