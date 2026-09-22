/**
 * The icon set, rendered inline.
 *
 * Inline rather than `<img>` or an external sprite, for three reasons that all turned out to
 * matter. An `<img>` cannot take `currentColor`, so every icon would need a copy per theme. An
 * external sprite ties `<use>` to an origin, and the Tauri build does not share one with the web
 * build. And inlining raw markup would mean rendering a string, which is a habit worth not
 * starting. The geometry is structured data, checked by the compiler, and the whole set is
 * 21 KB before tree-shaking.
 *
 * The one rule this file exists to enforce: **an icon is never alone**. See [`IconLabel`].
 */

import type { ReactElement } from 'react';

import { ICON_GRID, type IconName, icons } from './icons.generated';

export type { IconName };
export { icons };

/** How large the glyph is drawn. */
export type IconSize = 'small' | 'default' | 'large';

const SIZES: Record<IconSize, string> = {
  // Below the body text, for a glyph inside a line of prose.
  small: '1em',
  default: '1.35em',
  // For a primary action, where the target is 48 px and the glyph should not swim in it.
  large: '1.75em',
};

export interface IconProps {
  /** Which glyph. */
  readonly name: IconName;
  /** How large, relative to the surrounding text. */
  readonly size?: IconSize;
  /**
   * What a screen reader should say, when the glyph is not accompanied by visible text.
   *
   * Leave it unset inside an [`IconLabel`] — the label is already read, and repeating it makes
   * the control announce itself twice.
   */
  readonly title?: string;
  /** Extra classes, for positioning only. Colour comes from `currentColor`. */
  readonly className?: string;
}

/**
 * One glyph, coloured by the text around it.
 *
 * ```tsx
 * <Icon name="search" />
 * ```
 */
export function Icon({ name, size = 'default', title, className }: IconProps): ReactElement {
  const definition = icons[name];
  const extent = SIZES[size];

  return (
    <svg
      className={className}
      width={extent}
      height={extent}
      viewBox={`0 0 ${ICON_GRID} ${ICON_GRID}`}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      // Decorative by default: the visible label carries the meaning. A title overrides it.
      role={title ? 'img' : undefined}
      aria-hidden={title ? undefined : true}
      focusable={false}
    >
      {title ? <title>{title}</title> : null}
      {definition.shapes.map((shape, index) =>
        shape.kind === 'path' ? (
          // The index is the identity: the shapes of one glyph are fixed at build time and
          // never reorder.
          // eslint-disable-next-line react/no-array-index-key
          <path key={index} d={shape.d} />
        ) : (
          // eslint-disable-next-line react/no-array-index-key
          <circle key={index} cx={shape.cx} cy={shape.cy} r={shape.r} />
        ),
      )}
    </svg>
  );
}

export interface IconLabelProps {
  /** Which glyph. */
  readonly name: IconName;
  /**
   * The visible text.
   *
   * Defaults to the label that ships with the icon, which is the Russian one from plate 14.
   * Passing a different string is for context — "Добавить ребёнка" rather than "Добавить" —
   * and not for leaving it out.
   */
  readonly children?: string;
  /** How large the glyph is drawn. */
  readonly size?: IconSize;
  /** Extra classes. */
  readonly className?: string;
}

/**
 * A glyph with its label, which is the only form the interface uses.
 *
 * The evidence on older users is not ambiguous: an unlabelled icon is a guess, and a tooltip is
 * not a label because it needs a hover the primary persona may never perform. So the label is
 * part of the component rather than something each caller remembers, and the glyph is marked
 * decorative so a screen reader announces the control once.
 *
 * ```tsx
 * <button type="button"><IconLabel name="search" /></button>
 * ```
 */
export function IconLabel({ name, children, size, className }: IconLabelProps): ReactElement {
  return (
    <span className={className ? `dv-icon-label ${className}` : 'dv-icon-label'}>
      {/* `exactOptionalPropertyTypes` draws a real distinction here: "no size given" and
          "size given as undefined" are different, and only the first should fall back. */}
      <Icon name={name} {...(size ? { size } : {})} />
      <span>{children ?? icons[name].label}</span>
    </span>
  );
}
