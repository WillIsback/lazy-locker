"""
lazy-locker SDK for Python

Injecte les secrets du locker dans les variables d'environnement.
L'agent lazy-locker doit être démarré (lancez 'lazy-locker' et entrez votre passphrase).

Usage:
    from lazy_locker import inject_secrets
    inject_secrets()
    
    # Maintenant os.environ contient vos secrets
    import os
    api_key = os.environ["MY_API_KEY"]
"""

import json
import os
import socket
from pathlib import Path
from typing import Dict, Optional


def get_socket_path() -> Path:
    """Retourne le chemin du socket de l'agent."""
    config_dir = Path.home() / ".config" / ".lazy-locker"
    return config_dir / "agent.sock"


def _send_request(request: dict) -> dict:
    """Envoie une requête à l'agent et retourne la réponse."""
    socket_path = get_socket_path()
    
    if not socket_path.exists():
        raise ConnectionError(
            "Agent lazy-locker non démarré. "
            "Lancez 'lazy-locker' et entrez votre passphrase."
        )
    
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        sock.connect(str(socket_path))
        sock.sendall((json.dumps(request) + "\n").encode())
        
        # Lire la réponse
        response = b""
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            response += chunk
            if b"\n" in response:
                break
        
        return json.loads(response.decode().strip())
    finally:
        sock.close()


def is_agent_running() -> bool:
    """Vérifie si l'agent est en cours d'exécution."""
    try:
        response = _send_request({"action": "ping"})
        return response.get("status") == "ok"
    except (ConnectionError, FileNotFoundError, ConnectionRefusedError):
        return False


def get_secrets() -> Dict[str, str]:
    """
    Récupère tous les secrets depuis l'agent.
    
    Returns:
        Dict[str, str]: Dictionnaire nom -> valeur des secrets
        
    Raises:
        ConnectionError: Si l'agent n'est pas démarré
        RuntimeError: Si l'agent retourne une erreur
    """
    response = _send_request({"action": "get_secrets"})
    
    if response.get("status") == "ok":
        return response.get("data", {})
    else:
        raise RuntimeError(response.get("message", "Erreur inconnue"))


def get_secret(name: str) -> Optional[str]:
    """
    Récupère un secret spécifique depuis l'agent.
    
    Args:
        name: Nom du secret
        
    Returns:
        La valeur du secret ou None si non trouvé
    """
    response = _send_request({"action": "get_secret", "name": name})
    
    if response.get("status") == "ok":
        return response.get("data", {}).get("value")
    else:
        return None


def inject_secrets(prefix: str = "", override: bool = True) -> int:
    """
    Injecte tous les secrets dans os.environ.
    
    Args:
        prefix: Préfixe optionnel à ajouter aux noms de variables
        override: Si True, écrase les variables existantes
        
    Returns:
        int: Nombre de secrets injectés
        
    Raises:
        ConnectionError: Si l'agent n'est pas démarré
        
    Example:
        >>> from lazy_locker import inject_secrets
        >>> inject_secrets()
        3
        >>> import os
        >>> os.environ["MY_API_KEY"]
        'secret_value'
    """
    secrets = get_secrets()
    count = 0
    
    for name, value in secrets.items():
        env_name = f"{prefix}{name}" if prefix else name
        
        if override or env_name not in os.environ:
            os.environ[env_name] = value
            count += 1
    
    return count


def status() -> dict:
    """
    Retourne le statut de l'agent.
    
    Returns:
        dict: Informations sur l'agent (uptime, TTL restant, etc.)
    """
    response = _send_request({"action": "ping"})
    
    if response.get("status") == "ok":
        return response.get("data", {})
    else:
        raise RuntimeError(response.get("message", "Agent non disponible"))


# Alias pour compatibilité avec python-dotenv
load_secrets = inject_secrets
