/**
 * lazy-locker SDK for JavaScript/TypeScript
 *
 * Injecte les secrets du locker dans process.env.
 * L'agent lazy-locker doit être démarré (lancez 'lazy-locker' et entrez votre passphrase).
 *
 * @example
 * import { injectSecrets } from 'lazy-locker';
 * await injectSecrets();
 *
 * // Maintenant process.env contient vos secrets
 * const apiKey = process.env.MY_API_KEY;
 */

import { createConnection } from 'net';
import { homedir } from 'os';
import { join } from 'path';

interface AgentResponse {
  status: 'ok' | 'error';
  data?: Record<string, unknown>;
  message?: string;
}

/**
 * Retourne le chemin du socket de l'agent
 */
function getSocketPath(): string {
  return join(homedir(), '.config', '.lazy-locker', 'agent.sock');
}

/**
 * Envoie une requête à l'agent et retourne la réponse
 */
function sendRequest(request: Record<string, unknown>): Promise<AgentResponse> {
  return new Promise((resolve, reject) => {
    const socketPath = getSocketPath();

    const client = createConnection(socketPath, () => {
      client.write(JSON.stringify(request) + '\n');
    });

    let data = '';

    client.on('data', (chunk) => {
      data += chunk.toString();
      if (data.includes('\n')) {
        client.end();
      }
    });

    client.on('end', () => {
      try {
        resolve(JSON.parse(data.trim()));
      } catch {
        reject(new Error('Invalid response from agent'));
      }
    });

    client.on('error', (err) => {
      if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
        reject(
          new Error(
            "Agent lazy-locker non démarré. Lancez 'lazy-locker' et entrez votre passphrase."
          )
        );
      } else {
        reject(err);
      }
    });
  });
}

/**
 * Vérifie si l'agent est en cours d'exécution
 */
export async function isAgentRunning(): Promise<boolean> {
  try {
    const response = await sendRequest({ action: 'ping' });
    return response.status === 'ok';
  } catch {
    return false;
  }
}

/**
 * Récupère tous les secrets depuis l'agent
 *
 * @returns Dictionnaire nom -> valeur des secrets
 * @throws Si l'agent n'est pas démarré
 */
export async function getSecrets(): Promise<Record<string, string>> {
  const response = await sendRequest({ action: 'get_secrets' });

  if (response.status === 'ok') {
    return (response.data as Record<string, string>) || {};
  } else {
    throw new Error(response.message || 'Erreur inconnue');
  }
}

/**
 * Récupère un secret spécifique depuis l'agent
 *
 * @param name - Nom du secret
 * @returns La valeur du secret ou undefined si non trouvé
 */
export async function getSecret(name: string): Promise<string | undefined> {
  const response = await sendRequest({ action: 'get_secret', name });

  if (response.status === 'ok') {
    return (response.data as { value: string })?.value;
  }
  return undefined;
}

/**
 * Injecte tous les secrets dans process.env
 *
 * @param options - Options d'injection
 * @param options.prefix - Préfixe optionnel à ajouter aux noms de variables
 * @param options.override - Si true, écrase les variables existantes (défaut: true)
 * @returns Nombre de secrets injectés
 *
 * @example
 * import { injectSecrets } from 'lazy-locker';
 *
 * await injectSecrets();
 * console.log(process.env.MY_API_KEY);
 */
export async function injectSecrets(options: { prefix?: string; override?: boolean } = {}): Promise<number> {
  const { prefix = '', override = true } = options;
  const secrets = await getSecrets();
  let count = 0;

  for (const [name, value] of Object.entries(secrets)) {
    const envName = prefix ? `${prefix}${name}` : name;

    if (override || !(envName in process.env)) {
      process.env[envName] = value;
      count++;
    }
  }

  return count;
}

/**
 * Retourne le statut de l'agent
 *
 * @returns Informations sur l'agent (uptime, TTL restant, etc.)
 */
export async function status(): Promise<{ uptime_secs: number; ttl_remaining_secs: number }> {
  const response = await sendRequest({ action: 'ping' });

  if (response.status === 'ok') {
    return response.data as { uptime_secs: number; ttl_remaining_secs: number };
  } else {
    throw new Error(response.message || 'Agent non disponible');
  }
}

/**
 * Configuration automatique - charge les secrets au require/import
 * Usage: import 'lazy-locker/config'
 */
export async function config(): Promise<void> {
  await injectSecrets();
}

// Alias pour compatibilité
export const loadSecrets = injectSecrets;
