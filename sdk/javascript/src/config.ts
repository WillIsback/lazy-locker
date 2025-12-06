/**
 * Auto-configuration module
 * Usage: import 'lazy-locker/config'
 */

import { injectSecrets } from './index';

// Injection automatique au chargement du module
injectSecrets().catch((err) => {
  console.error('[lazy-locker] Erreur:', err.message);
});
