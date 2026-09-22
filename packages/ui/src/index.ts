/**
 * The design system.
 *
 * Everything the interface draws with that is not a screen: the icon set, and the components
 * that will accumulate here over Phase 1. Colour never appears in this package as a literal —
 * it comes from `@drevigen/tokens`, and the artwork gate refuses a build where it does not.
 */

export { Icon, IconLabel, icons } from './Icon';
export type { IconLabelProps, IconName, IconProps, IconSize } from './Icon';
export { ICON_GRID } from './icons.generated';
export type { IconDefinition, IconShape } from './icons.generated';
