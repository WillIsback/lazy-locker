/**
 * Script de test pour vérifier l'injection des secrets via le SDK lazy-locker.
 *
 * Usage:
 *   1. Lancez lazy-locker et entrez votre passphrase (l'agent démarre automatiquement)
 *   2. bun run test_env.ts (ou npx tsx test_env.ts)
 */

import { injectSecrets, isAgentRunning, status, getSecrets } from '../sdk/javascript/src/index';

console.log('='.repeat(50));
console.log('Test du SDK lazy-locker pour JavaScript/TypeScript');
console.log('='.repeat(50));

async function main() {
  // Vérifier si l'agent est actif
  if (!(await isAgentRunning())) {
    console.log('\n❌ Agent non démarré!');
    console.log("   Lancez 'lazy-locker' et entrez votre passphrase.");
    process.exit(1);
  }

  // Afficher le statut
  try {
    const info = await status();
    const remainingHours = Math.floor(info.ttl_remaining_secs / 3600);
    const remainingMins = Math.floor((info.ttl_remaining_secs % 3600) / 60);
    console.log(`\n✅ Agent actif (TTL: ${remainingHours}h ${remainingMins}m)`);
  } catch (e) {
    console.log(`\n⚠️  Erreur statut: ${e}`);
  }

  // Injecter les secrets
  try {
    const count = await injectSecrets();
    console.log(`✅ ${count} secrets injectés dans process.env`);
  } catch (e) {
    console.log(`\n❌ Erreur injection: ${e}`);
    process.exit(1);
  }

  // Afficher les secrets (masqués)
  console.log('\nSecrets disponibles:');
  const secrets = await getSecrets();
  for (const [key, value] of Object.entries(secrets)) {
    const masked = value.length > 3 ? value.slice(0, 3) + '*'.repeat(value.length - 3) : '***';
    console.log(`  ${key} = ${masked}`);
  }

  console.log('\n' + '='.repeat(50));
  console.log('✅ Test réussi! Les secrets sont injectés.');
  console.log('='.repeat(50));
}

main().catch(console.error);
