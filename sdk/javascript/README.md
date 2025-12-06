# lazy-locker SDK for JavaScript/TypeScript

SDK JavaScript/TypeScript pour [lazy-locker](https://github.com/lazy-locker/lazy-locker) - Gestionnaire de secrets sécurisé.

## Installation

```bash
npm install lazy-locker
# ou
bun add lazy-locker
# ou
pnpm add lazy-locker
```

## Prérequis

L'agent lazy-locker doit être démarré :

```bash
lazy-locker  # Entrez votre passphrase dans le TUI
```

L'agent reste actif pendant 8 heures.

## Usage

### Injection automatique

```typescript
import { injectSecrets } from 'lazy-locker';

// Injecte tous les secrets dans process.env
await injectSecrets();

// Utilisez vos secrets normalement
const apiKey = process.env.MY_API_KEY;
```

### Configuration one-liner

```typescript
// En haut de votre fichier d'entrée
import 'lazy-locker/config';
```

### Récupération manuelle

```typescript
import { getSecrets, getSecret } from 'lazy-locker';

// Tous les secrets
const secrets = await getSecrets();
console.log(secrets); // { MY_API_KEY: "xxx", DB_PASSWORD: "yyy" }

// Un secret spécifique
const apiKey = await getSecret('MY_API_KEY');
```

### Vérification de l'agent

```typescript
import { isAgentRunning, status } from 'lazy-locker';

if (await isAgentRunning()) {
  const info = await status();
  console.log(`Agent actif, TTL restant: ${info.ttl_remaining_secs}s`);
} else {
  console.log('Lancez lazy-locker pour démarrer l\'agent');
}
```

## Comparaison avec dotenv

| Feature | dotenv | lazy-locker |
|---------|--------|-------------|
| Secrets en clair sur disque | ✅ Oui (.env) | ❌ Non (chiffré) |
| Versioning sécurisé | ❌ Non | ✅ Oui |
| Expiration des secrets | ❌ Non | ✅ Oui |
| Multi-projet | ❌ Non | ✅ Oui |

## Migration depuis dotenv

```typescript
// Avant
import 'dotenv/config';

// Après
import 'lazy-locker/config';

// Le reste du code reste identique !
```

## TypeScript

Le SDK inclut les définitions TypeScript. Aucune configuration supplémentaire n'est nécessaire.

```typescript
import { getSecrets } from 'lazy-locker';

const secrets: Record<string, string> = await getSecrets();
```
