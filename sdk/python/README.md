# lazy-locker SDK for Python

SDK Python pour [lazy-locker](https://github.com/lazy-locker/lazy-locker) - Gestionnaire de secrets sécurisé.

## Installation

```bash
pip install lazy-locker
# ou
uv add lazy-locker
```

## Prérequis

L'agent lazy-locker doit être démarré :

```bash
lazy-locker  # Entrez votre passphrase dans le TUI
```

L'agent reste actif pendant 8 heures.

## Usage

### Injection automatique

```python
from lazy_locker import inject_secrets
import os

# Injecte tous les secrets dans os.environ
inject_secrets()

# Utilisez vos secrets normalement
api_key = os.environ["MY_API_KEY"]
```

### Récupération manuelle

```python
from lazy_locker import get_secrets, get_secret

# Tous les secrets
secrets = get_secrets()
print(secrets)  # {"MY_API_KEY": "xxx", "DB_PASSWORD": "yyy"}

# Un secret spécifique
api_key = get_secret("MY_API_KEY")
```

### Vérification de l'agent

```python
from lazy_locker import is_agent_running, status

if is_agent_running():
    info = status()
    print(f"Agent actif, TTL restant: {info['ttl_remaining_secs']}s")
else:
    print("Lancez lazy-locker pour démarrer l'agent")
```

## Comparaison avec python-dotenv

| Feature | python-dotenv | lazy-locker |
|---------|---------------|-------------|
| Secrets en clair sur disque | ✅ Oui (.env) | ❌ Non (chiffré) |
| Versioning sécurisé | ❌ Non | ✅ Oui |
| Expiration des secrets | ❌ Non | ✅ Oui |
| Multi-projet | ❌ Non | ✅ Oui |

## Migration depuis python-dotenv

```python
# Avant
from dotenv import load_dotenv
load_dotenv()

# Après
from lazy_locker import inject_secrets
inject_secrets()

# Le reste du code reste identique !
```
