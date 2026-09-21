/**
 * The web entry point.
 *
 * It does two things the Tauri entry point does not: it registers the service worker that makes
 * the application installable, and it applies the remembered theme before React mounts. The
 * second matters more than it looks — applying the theme in an effect means the first painted
 * frame is the wrong one, and a flash of the light theme at night is exactly the kind of small
 * insult this project is trying not to commit.
 */

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { App } from '@drevigen/app';
import { applyTextScale, applyTheme, readTextScale, readTheme } from '@drevigen/app/platform';

applyTheme(readTheme());
applyTextScale(readTextScale());

const container = document.getElementById('root');
if (!container) {
  throw new Error('index.html is missing #root; the page cannot mount');
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
