#!/usr/bin/env python3
"""
Script de test pour vérifier l'injection des secrets via le SDK lazy-locker.

Usage:
  1. Lancez lazy-locker et entrez votre passphrase (l'agent démarre automatiquement)
  2. python test_env.py (ou uv run test_env.py, bun run test_env.py, etc.)
"""

import os
import sys

# Add SDK to path for local testing
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'sdk', 'python'))

from lazy_locker import inject_secrets, is_agent_running, status

print("=" * 50)
print("Test du SDK lazy-locker pour Python")
print("=" * 50)

# Vérifier si l'agent est actif
if not is_agent_running():
    print("\n❌ Agent non démarré!")
    print("   Lancez 'lazy-locker' et entrez votre passphrase.")
    sys.exit(1)

# Afficher le statut
try:
    info = status()
    remaining_hours = info.get('ttl_remaining_secs', 0) // 3600
    remaining_mins = (info.get('ttl_remaining_secs', 0) % 3600) // 60
    print(f"\n✅ Agent actif (TTL: {remaining_hours}h {remaining_mins}m)")
except Exception as e:
    print(f"\n⚠️  Erreur statut: {e}")

# Injecter les secrets
try:
    count = inject_secrets()
    print(f"✅ {count} secrets injectés dans os.environ")
except Exception as e:
    print(f"\n❌ Erreur injection: {e}")
    sys.exit(1)

# Afficher les secrets (masqués)
print("\nSecrets disponibles:")
for key in ['test', 'test2', 'MY_API_KEY', 'DB_PASSWORD']:
    value = os.getenv(key)
    if value:
        masked = value[:3] + '*' * (len(value) - 3) if len(value) > 3 else '***'
        print(f"  {key} = {masked}")
    else:
        print(f"  {key} = (non défini)")

print("\n" + "=" * 50)
print("✅ Test réussi! Les secrets sont injectés.")
print("=" * 50)