# Plan d'Implementation - Voice System & Hybrid Memory Search

**Decision utilisateur** : Voice Mode avec wake word + Hybrid Memory Search

---

## 1. Vue d'Ensemble

| Feature | Description | Complexite |
|---------|-------------|------------|
| **Voice Mode Toggle** | Badge "Vocal" vert/gris avec wake word + talk mode | Haute |
| **Hybrid Memory Search** | BM25 + Semantic search combines | Moyenne |

---

## 2. Voice System - UX Design

### 2.1 Interface Utilisateur

**Badge "Vocal" dans le header du chat** (entre "en ligne" et "supprimer") :

```
┌────────────────────────────────────────────────────────────────┐
│  [Avatar] Compagnon    🟢 En ligne   [🎤 Vocal]   [🗑 Suppr.]  │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  MODE VOCAL ACTIF (badge vert) :                               │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                                                          │  │
│  │        🎤  J'ecoute... dis "Hey Compagnon"               │  │
│  │            ~~~~~~~~~~~~ (animation onde)                 │  │
│  │                                                          │  │
│  │  [Cliquez pour desactiver le mode vocal]                 │  │
│  └──────────────────────────────────────────────────────────┘  │
│  (Champ de saisie texte MASQUE)                                │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  MODE VOCAL INACTIF (badge gris) :                             │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ [Tapez votre message...              ] [🎤] [Envoyer →]  │  │
│  └──────────────────────────────────────────────────────────┘  │
│  (Interface actuelle preservee)                                │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

**Etats du badge** :
| Etat | Couleur | Comportement |
|------|---------|--------------|
| Inactif | Gris | Mode texte normal, clic active le mode vocal |
| Actif (ecoute) | Vert | Ecoute wake word, clic desactive |
| Actif (detection) | Vert clignotant | Wake word detecte, enregistrement en cours |
| Actif (processing) | Vert pulse | Transcription/LLM/TTS en cours |

### 2.2 Architecture Full Sherpa-onnx (WASM KWS + Python STT)

**IMPORTANT** : Architecture 100% offline, gratuite, multi-langue, **mono-technologie Sherpa-onnx**.

| Composant | Technologie | Justification |
|-----------|-------------|---------------|
| Wake Word | **Sherpa-onnx WASM KWS** | Keyword Spotting dans browser, gratuit, open vocabulary |
| STT | **Sherpa-onnx** (Python) | Offline, multi-langue, rapide sur CPU |
| Transport | **WebSocket** `/ws/audio` | Streaming audio temps reel |
| Modele KWS | **zipformer-gigaspeech-3.3M** | English, modele leger ~3MB |
| Modele STT | **SenseVoiceSmall** | FR/EN/DE/ES/IT/ZH/JA/KO en un seul modele |

**Avantage Architecture Full Sherpa-onnx** :
- **Mono-technologie** : Un seul framework (Sherpa-onnx) pour KWS et STT
- **Open Vocabulary** : N'importe quel mot-cle configurable sans reentrainement
- **Code officiel** : Utilise l'exemple WASM/KWS du repo Sherpa-onnx

```
┌─────────────────────────────────────────────────────────────────────┐
│           VOICE MODE FLOW (Full Sherpa-onnx Architecture)            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  FRONTEND (Browser) - Sherpa-onnx WASM                               │
│  ┌─────────────────────┐                                             │
│  │ 1. OS Permission    │  Premier usage : browser demande micro      │
│  │    (si necessaire)  │  - Permission persistee par browser         │
│  └─────────┬───────────┘                                             │
│            │                                                         │
│            ▼                                                         │
│  ┌─────────────────────┐                                             │
│  │ 2. Sherpa-onnx KWS  │  WASM Keyword Spotting dans le browser      │
│  │    (WASM)           │  - Modele: zipformer-gigaspeech-3.3M        │
│  │    "Hey Sherpa"     │  - Open vocabulary: mot-cle configurable    │
│  │                     │  - Latence <100ms, 100% offline             │
│  └─────────┬───────────┘  - CDN jsdelivr + modele depuis GitHub      │
│            │ (keyword detecte)                                       │
│            ▼                                                         │
│  ┌─────────────────────┐                                             │
│  │ 3. WEBSOCKET OPEN   │  Connexion /ws/audio                        │
│  │    + AUDIO STREAM   │  - Format: PCM 16kHz mono                   │
│  │                     │  - Envoi chunks audio en temps reel         │
│  └─────────┬───────────┘  - VAD cote frontend pour fin de phrase     │
│            │                                                         │
│────────────┼─────────────────────────────────────────────────────────│
│            │                                                         │
│  BACKEND (Python) - Sherpa-onnx                                      │
│            ▼                                                         │
│  ┌─────────────────────┐                                             │
│  │ 4. SHERPA-ONNX STT  │  OfflineRecognizer                          │
│  │    (SenseVoiceSmall)│  - Modele unique multi-langue               │
│  │                     │  - FR/EN/DE/ES/IT/ZH/JA/KO                  │
│  │                     │  - Ultra-rapide sur CPU (Pi compatible)     │
│  └─────────┬───────────┘  - GRATUIT, pas de cout API                 │
│            │                                                         │
│            ▼                                                         │
│  ┌─────────────────────┐                                             │
│  │ 5. CHAT + TTS       │  Flow existant                              │
│  │                     │  - SSE streaming + TTS response             │
│  └─────────┬───────────┘                                             │
│            │                                                         │
│────────────┼─────────────────────────────────────────────────────────│
│            │                                                         │
│  FRONTEND  ▼                                                         │
│  ┌─────────────────────┐                                             │
│  │ 6. RETOUR ECOUTE    │  Cycle Talk Mode                            │
│  │  (Sherpa-onnx KWS)  │  - Apres TTS, retour detection keyword      │
│  └─────────────────────┘  - User peut interrompre TTS en parlant     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**Avantages Architecture** :
- **100% Gratuit** : Aucun cout API (vs Whisper $0.006/min)
- **Offline** : Fonctionne sans internet (apres chargement initial)
- **Multi-langue** : Un seul modele pour 8+ langues
- **Performant** : Sherpa-onnx optimise pour CPU (Raspberry Pi)
- **Privacy** : Audio traite localement, jamais envoye au cloud
- **Multi-arch** : Windows (AMD64) dev + Raspberry Pi (ARM64) prod

### 2.3 Parametres Utilisateur (Settings)

```typescript
// User voice preferences (stored in DB)
interface VoiceSettings {
  voiceModeEnabled: boolean;           // Toggle global (defaut: false)
  wakeWord: string;                    // Defaut: "Hey Sherpa" (anglais, GigaSpeech)
  wakeWordSensitivity: number;         // 0.5 - 1.0 (defaut: 0.8)
  vadSilenceThreshold: number;         // ms de silence pour fin de phrase (defaut: 1000)
}

// NOTE: Parametres NON inclus (utiliser existants) :
// - autoPlayTTS: Utiliser le systeme TTS existant
// - voiceLanguage: Utiliser User.language existant
```

**Endpoint settings** :
```
GET/PATCH /api/v1/auth/me/voice-settings
```

### 2.4 Backend Sherpa-onnx STT (GOLD GRADE)

**IMPORTANT** : STT 100% gratuit avec Sherpa-onnx. Pas de tracking de cout necessaire.

> **Patterns appliques** : Singleton, logging structure, metrics, exceptions, async non-bloquant

#### 2.4.1 Exceptions (core/exceptions.py - AJOUT)

```python
class STTError(BaseAPIException):
    """Error during speech-to-text transcription."""
    def __init__(
        self,
        detail: str = "Transcription failed",
        **log_context: Any,
    ) -> None:
        super().__init__(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=detail,
            log_level="error",
            log_event="stt_error",
            **log_context,
        )

class WebSocketAuthError(BaseAPIException):
    """WebSocket authentication failed."""
    def __init__(self, detail: str = "Unauthorized") -> None:
        super().__init__(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail=detail,
            log_level="warning",
            log_event="websocket_auth_failed",
        )
```

#### 2.4.2 Metrics (infrastructure/observability/metrics.py - AJOUT)

```python
# STT Metrics
stt_transcriptions_total = Counter(
    "voice_stt_transcriptions_total",
    "Total STT transcriptions",
    ["status"],  # success, error, timeout
)

stt_transcription_duration_seconds = Histogram(
    "voice_stt_transcription_duration_seconds",
    "STT processing time (not audio duration)",
    buckets=[0.1, 0.5, 1.0, 2.0, 5.0, 10.0],
)

stt_audio_duration_seconds = Histogram(
    "voice_stt_audio_duration_seconds",
    "Audio duration received for transcription",
    buckets=[1, 5, 10, 30, 60],
)

websocket_connections_active = Gauge(
    "voice_websocket_connections_active",
    "Active audio WebSocket connections",
)
```

#### 2.4.3 STT Service (domains/voice/stt/sherpa_stt.py - CREER)

```python
"""
Sherpa-onnx Speech-to-Text Service.

Provides offline, multi-language transcription using SenseVoiceSmall model.
Follows codebase patterns: singleton, structured logging, metrics.
"""
import asyncio
from concurrent.futures import ThreadPoolExecutor
from functools import lru_cache
from pathlib import Path
from typing import TYPE_CHECKING

import sherpa_onnx

from src.core.config import get_settings
from src.core.exceptions import STTError
from src.infrastructure.observability.logging import get_logger
from src.infrastructure.observability.metrics import (
    stt_transcriptions_total,
    stt_transcription_duration_seconds,
    stt_audio_duration_seconds,
)

if TYPE_CHECKING:
    from src.core.config import Settings

logger = get_logger(__name__)

# Thread pool for CPU-bound STT (avoid blocking event loop)
_stt_executor = ThreadPoolExecutor(max_workers=4, thread_name_prefix="stt")


class SherpaSttService:
    """
    Speech-to-Text service using Sherpa-onnx OfflineRecognizer.

    Model: SenseVoiceSmall (multi-langue: FR/EN/DE/ES/IT/ZH/JA/KO)
    Thread-safe via ThreadPoolExecutor for async operations.

    Usage:
        stt = get_stt_service()
        text = await stt.transcribe_async(audio_samples)
    """

    def __init__(self, settings: "Settings") -> None:
        model_path = Path(settings.sherpa_model_path)

        if not model_path.exists():
            raise STTError(
                f"STT model not found at {model_path}",
                model_path=str(model_path),
            )

        self._recognizer = sherpa_onnx.OfflineRecognizer.from_sense_voice(
            model=str(model_path / "model.onnx"),
            tokens=str(model_path / "tokens.txt"),
            num_threads=settings.sherpa_num_threads,
            use_itn=settings.sherpa_use_itn,
        )

        logger.info(
            "stt_service_initialized",
            model_path=str(model_path),
            num_threads=settings.sherpa_num_threads,
        )

    def transcribe(self, audio_samples: list[float], sample_rate: int = 16000) -> str:
        """
        Transcribe audio samples to text (SYNCHRONOUS).

        WARNING: This blocks the thread. Use transcribe_async() in async context.

        Args:
            audio_samples: PCM float samples normalized [-1, 1]
            sample_rate: Sample rate (must be 16000)

        Returns:
            Transcribed text
        """
        stream = self._recognizer.create_stream()
        stream.accept_waveform(sample_rate, audio_samples)
        self._recognizer.decode_stream(stream)
        return stream.result.text.strip()

    async def transcribe_async(
        self,
        audio_samples: list[float],
        sample_rate: int = 16000,
    ) -> str:
        """
        Transcribe audio samples to text (ASYNC, non-blocking).

        Uses ThreadPoolExecutor to avoid blocking the event loop.

        Args:
            audio_samples: PCM float samples normalized [-1, 1]
            sample_rate: Sample rate (must be 16000)

        Returns:
            Transcribed text

        Raises:
            STTError: On transcription failure
        """
        audio_duration = len(audio_samples) / sample_rate
        stt_audio_duration_seconds.observe(audio_duration)

        try:
            with stt_transcription_duration_seconds.time():
                loop = asyncio.get_event_loop()
                text = await loop.run_in_executor(
                    _stt_executor,
                    self.transcribe,
                    audio_samples,
                    sample_rate,
                )

            stt_transcriptions_total.labels(status="success").inc()
            logger.debug(
                "stt_transcription_completed",
                audio_duration_seconds=audio_duration,
                text_length=len(text),
            )
            return text

        except Exception as e:
            stt_transcriptions_total.labels(status="error").inc()
            logger.error(
                "stt_transcription_failed",
                audio_duration_seconds=audio_duration,
                error=str(e),
            )
            raise STTError(f"Transcription failed: {e}") from e


@lru_cache
def get_stt_service() -> SherpaSttService:
    """Get singleton SherpaSttService instance."""
    return SherpaSttService(get_settings())
```

#### 2.4.4 WebSocket Ticket System (BFF Pattern - CORRECTION MAJEURE)

**IMPORTANT**: Le codebase utilise le **BFF pattern** avec cookies de session HTTP-only.
`verify_access_token()` n'existe PAS. Solution: **Ticket System** à durée de vie courte.

##### A. Ticket Store (domains/voice/ticket_store.py - CREER)

```python
"""
WebSocket ticket store for BFF pattern authentication.

WebSocket cannot use HTTP-only cookies directly. This ticket system provides:
1. Short-lived tokens (60s TTL)
2. Single-use (deleted after validation)
3. Ties to authenticated session via REST endpoint

Pattern: Similar to OAuth state storage (core/oauth/flow_handler.py)
"""
import json
from uuid import uuid4

import redis.asyncio as aioredis
import structlog

from src.core.config import settings

logger = structlog.get_logger(__name__)

# Ticket TTL in seconds (short-lived for security)
WS_TICKET_TTL_SECONDS = 60


class WebSocketTicketStore:
    """
    Short-lived ticket store for WebSocket authentication.

    Flow:
    1. User calls POST /api/v1/voice/ticket (authenticated via session cookie)
    2. Generates ticket, stores in Redis with 60s TTL
    3. Returns ticket to frontend
    4. Frontend connects to WebSocket with ?ticket=xxx
    5. WebSocket validates ticket, deletes it (single-use), proceeds

    Security:
    - Tickets are single-use (deleted after validation)
    - 60 second TTL (short window for replay attacks)
    - Tied to user_id from authenticated session
    """

    def __init__(self, redis_client: aioredis.Redis) -> None:
        self.redis = redis_client

    async def create_ticket(self, user_id: str) -> str:
        """
        Create a short-lived WebSocket authentication ticket.

        Args:
            user_id: Authenticated user's UUID

        Returns:
            Ticket string (UUID)
        """
        ticket = str(uuid4())
        key = f"ws_ticket:{ticket}"

        await self.redis.setex(
            key,
            WS_TICKET_TTL_SECONDS,
            json.dumps({"user_id": user_id}),
        )

        logger.debug(
            "websocket_ticket_created",
            user_id=user_id,
            ticket_prefix=ticket[:8],
            ttl_seconds=WS_TICKET_TTL_SECONDS,
        )

        return ticket

    async def validate_and_consume_ticket(self, ticket: str) -> str | None:
        """
        Validate ticket and consume it (single-use).

        Args:
            ticket: Ticket string from WebSocket query param

        Returns:
            user_id if valid, None if invalid/expired/already-used
        """
        key = f"ws_ticket:{ticket}"

        # GET and DELETE atomically (pipeline)
        pipe = self.redis.pipeline()
        pipe.get(key)
        pipe.delete(key)
        results = await pipe.execute()

        data = results[0]  # GET result
        deleted = results[1]  # DELETE result

        if not data:
            logger.warning(
                "websocket_ticket_invalid",
                ticket_prefix=ticket[:8] if ticket else "none",
                reason="not_found_or_expired",
            )
            return None

        try:
            ticket_data = json.loads(data)
            user_id = ticket_data["user_id"]

            logger.debug(
                "websocket_ticket_validated",
                user_id=user_id,
                ticket_prefix=ticket[:8],
                consumed=deleted > 0,
            )

            return user_id

        except (json.JSONDecodeError, KeyError) as e:
            logger.error(
                "websocket_ticket_parse_error",
                ticket_prefix=ticket[:8],
                error=str(e),
            )
            return None
```

##### B. Ticket Endpoint (domains/voice/router.py - AJOUTER)

```python
"""
Voice API endpoints including WebSocket ticket generation.
"""
from fastapi import APIRouter, Depends
from pydantic import BaseModel

from src.core.session_dependencies import get_current_active_session
from src.domains.auth.models import User
from src.domains.voice.ticket_store import WebSocketTicketStore
from src.infrastructure.cache.redis import get_redis_session
from src.infrastructure.observability.logging import get_logger

logger = get_logger(__name__)
router = APIRouter(prefix="/voice", tags=["voice"])


class WebSocketTicketResponse(BaseModel):
    """Response for WebSocket ticket creation."""
    ticket: str
    ttl_seconds: int = 60


@router.post("/ticket", response_model=WebSocketTicketResponse)
async def create_websocket_ticket(
    user: User = Depends(get_current_active_session),
) -> WebSocketTicketResponse:
    """
    Generate a short-lived ticket for WebSocket authentication.

    This endpoint is authenticated via session cookie (BFF pattern).
    Returns a ticket that can be used once to connect to /ws/audio.

    Flow:
    1. Frontend calls this endpoint (with session cookie)
    2. Gets ticket (valid 60 seconds)
    3. Connects to WebSocket: /ws/audio?ticket=xxx
    """
    redis = await get_redis_session()
    ticket_store = WebSocketTicketStore(redis)

    ticket = await ticket_store.create_ticket(str(user.id))

    logger.info(
        "websocket_ticket_issued",
        user_id=str(user.id),
        ticket_prefix=ticket[:8],
    )

    return WebSocketTicketResponse(ticket=ticket)
```

##### C. WebSocket Endpoint (domains/voice/router.py - MODIFIER)

```python
"""
Voice WebSocket endpoint for real-time audio transcription.

Follows codebase patterns: BFF ticket auth, rate limiting, structured logging, metrics.
"""
from fastapi import WebSocket, Query
import numpy as np

from src.domains.voice.stt.sherpa_stt import get_stt_service
from src.domains.voice.ticket_store import WebSocketTicketStore
from src.infrastructure.cache.redis import get_redis_cache, get_redis_session
from src.infrastructure.observability.logging import get_logger
from src.infrastructure.observability.metrics import websocket_connections_active
from src.infrastructure.rate_limiting.redis_limiter import RedisRateLimiter

logger = get_logger(__name__)

# Rate limit constants (WebSocket audio connections)
WS_AUDIO_RATE_LIMIT_MAX_CALLS = 10
WS_AUDIO_RATE_LIMIT_WINDOW_SECONDS = 60


@router.websocket("/ws/audio")
async def websocket_audio(
    websocket: WebSocket,
    ticket: str = Query(..., description="WebSocket authentication ticket"),
):
    """
    WebSocket endpoint for real-time audio transcription.

    Authentication: Ticket from POST /api/v1/voice/ticket (BFF pattern).
    Rate Limited: 10 connections/minute per user.

    Protocol:
    1. Call POST /api/v1/voice/ticket to get ticket
    2. Connect with ?ticket=<ticket>
    3. Send audio chunks (binary: PCM 16kHz mono, int16)
    4. Send text "END" when done speaking
    5. Receive JSON: {"type": "transcription", "text": "...", "duration_seconds": ...}
    """
    # 1. Authenticate via ticket (BFF pattern)
    redis_session = await get_redis_session()
    ticket_store = WebSocketTicketStore(redis_session)

    user_id = await ticket_store.validate_and_consume_ticket(ticket)
    if not user_id:
        logger.warning("websocket_auth_failed", reason="invalid_ticket")
        await websocket.close(code=4001, reason="Invalid or expired ticket")
        return

    # 2. Rate limit check
    try:
        redis_cache = await get_redis_cache()
        limiter = RedisRateLimiter(redis_cache)
        rate_limit_key = f"ws:audio:{user_id}"
        allowed = await limiter.acquire(
            key=rate_limit_key,
            max_calls=WS_AUDIO_RATE_LIMIT_MAX_CALLS,
            window_seconds=WS_AUDIO_RATE_LIMIT_WINDOW_SECONDS,
        )
        if not allowed:
            logger.warning("websocket_rate_limited", user_id=user_id)
            await websocket.close(code=4029, reason="Rate limited")
            return
    except Exception as e:
        # Fail open on Redis error (availability > strict rate limiting)
        logger.warning("websocket_rate_limit_error", error=str(e))

    # 3. Accept connection
    await websocket.accept()
    websocket_connections_active.inc()
    logger.info("websocket_connected", user_id=user_id)

    stt_service = get_stt_service()
    audio_buffer: list[bytes] = []

    try:
        while True:
            data = await websocket.receive()

            if "text" in data and data["text"] == "END":
                # End of audio - transcribe
                if audio_buffer:
                    # Convert int16 -> float32 normalized
                    audio_np = np.frombuffer(b"".join(audio_buffer), dtype=np.int16)
                    audio_float = audio_np.astype(np.float32) / 32768.0

                    # Transcribe (async, non-blocking)
                    text = await stt_service.transcribe_async(audio_float.tolist())

                    await websocket.send_json({
                        "type": "transcription",
                        "text": text,
                        "duration_seconds": len(audio_float) / 16000,
                    })

                audio_buffer = []

            elif "bytes" in data:
                # Audio chunk received
                audio_buffer.append(data["bytes"])

            elif "text" in data and data["text"] == "PING":
                # Heartbeat
                await websocket.send_json({"type": "pong"})

    except Exception as e:
        logger.error("websocket_error", user_id=user_id, error=str(e))

    finally:
        websocket_connections_active.dec()
        logger.info("websocket_disconnected", user_id=user_id)
```

#### Dockerfile Multi-arch

```dockerfile
# Dockerfile.stt
FROM python:3.12-slim AS base

# Telecharger le modele SenseVoiceSmall
RUN apt-get update && apt-get install -y wget && \
    wget https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.10.16/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2 && \
    tar -xjf sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2 && \
    mv sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17 /models/sensevoice && \
    rm sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2

# Installer sherpa-onnx (wheels precompiles AMD64/ARM64)
RUN pip install sherpa-onnx

# Le reste de l'app...
```

#### Frontend : Modification ChatMessage.tsx

Puisque STT est gratuit, on affiche juste l'indicateur vocal sans cout :

```tsx
// ChatMessage.tsx - Section bulles user (ligne ~148)
<span className="text-[11px] ...">
  {formatTime(message.timestamp)}
  {/* Indicateur source vocale (pas de cout, STT gratuit) */}
  {message.source === 'voice' && (
    <span className="hidden mobile:inline">
      {' | '}
      <span className="text-purple-500">
        🎤 {message.audioDurationSeconds?.toFixed(1)}s
      </span>
    </span>
  )}
</span>
```

**Type Message a etendre** :
```typescript
// types/chat.ts
interface Message {
  // ... existant
  source?: 'text' | 'voice';           // NOUVEAU
  audioDurationSeconds?: number;        // NOUVEAU (pour info, pas de cout)
}
```

#### Affichage Final

```
┌──────────────────────────────────────────────────────────────┐
│ [User bubble]                                                │
│ "Quel temps fait-il demain a Paris ?"                       │
│                                                              │
│ 14:32 | lundi 28 janvier 2026 | 🎤 2.3s                     │
└──────────────────────────────────────────────────────────────┘
│
▼
┌──────────────────────────────────────────────────────────────┐
│ [Assistant bubble]                                           │
│ "Demain a Paris il fera 22°C avec un ciel degage..."        │
│                                                              │
│ 14:32 | ... | 🟠 150 IN 🟢 89 OUT 🔵 45 CACHE • 0.002€      │
└──────────────────────────────────────────────────────────────┘
```

**Note** : Pas de cout STT affiche car Sherpa-onnx est 100% gratuit.

### 2.5 Gestion des Erreurs et Edge Cases

| Scenario | Comportement |
|----------|--------------|
| **Micro non autorise** | Toast "Autorisez le micro pour le mode vocal" + retour mode texte |
| **Wake word non detecte** | Indicateur "J'ecoute..." avec timeout configurable (60s defaut) |
| **STT timeout** | Toast "Erreur de transcription" + retour ecoute wake word |
| **User interrompt TTS** | Stop TTS immediatement + ecoute nouvelle commande (voir 2.5.6) |
| **Bruit ambiant** | VAD filtre, ne declenche pas si pas de parole claire |
| **Connexion perdue** | Toast + retour mode texte automatique |
| **WebSocket deconnecte** | Reconnexion automatique avec backoff exponentiel (voir 2.5.7) |
| **Modele STT non charge** | Toast erreur + fallback mode texte |

### 2.5.6 Interruption TTS (Barge-In)

**Probleme** : Comment detecter que l'utilisateur parle pendant que le TTS joue ?

**Solution** : Sherpa-onnx KWS continue d'ecouter pendant le TTS :

```typescript
// useVoiceMode.ts - State machine
const states = {
  IDLE: 'idle',           // Mode texte
  LISTENING: 'listening', // Ecoute keyword
  RECORDING: 'recording', // Enregistre apres keyword detecte
  PROCESSING: 'processing', // STT en cours
  SPEAKING: 'speaking',   // TTS joue (MAIS KWS actif)
};

// Pendant SPEAKING, si keyword detecte -> interrompre TTS
onKeywordDetected: () => {
  if (state === 'SPEAKING') {
    stopTTS();  // Arrete la lecture audio
    setState('RECORDING');  // Passe en enregistrement
  }
}
```

**Avantage** : Pas besoin de VAD complexe pendant TTS, le keyword suffit.

### 2.5.7 WebSocket avec Ticket System (BFF Pattern)

```typescript
// hooks/useAudioWebSocket.ts
'use client';

import { useCallback, useRef, useState } from 'react';
import { logger } from '@/lib/logger';

const API_BASE = process.env.NEXT_PUBLIC_API_URL ?? '';
const WS_BASE = API_BASE.replace(/^http/, 'ws');
const RECONNECT_DELAYS = [1000, 2000, 4000, 8000, 16000];

interface UseAudioWebSocketOptions {
  onTranscription: (text: string, durationSeconds: number) => void;
  onError?: (error: Error) => void;
}

interface UseAudioWebSocketReturn {
  isConnected: boolean;
  isConnecting: boolean;
  error: Error | null;
  connect: () => Promise<void>;
  disconnect: () => void;
  sendAudio: (chunk: ArrayBuffer) => void;
  endAudio: () => void;
}

export function useAudioWebSocket({
  onTranscription,
  onError,
}: UseAudioWebSocketOptions): UseAudioWebSocketReturn {
  const wsRef = useRef<WebSocket | null>(null);
  const heartbeatRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const attemptRef = useRef(0);

  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  // Step 1: Get ticket from BFF (authenticated via session cookie)
  const getTicket = async (): Promise<string> => {
    const response = await fetch(`${API_BASE}/api/v1/voice/ticket`, {
      method: 'POST',
      credentials: 'include', // Send session cookie
    });

    if (!response.ok) {
      throw new Error(`Ticket request failed: ${response.status}`);
    }

    const data = await response.json();
    return data.ticket;
  };

  // Step 2: Connect WebSocket with ticket
  const connect = useCallback(async () => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return; // Already connected
    }

    setIsConnecting(true);
    setError(null);

    try {
      // Get ticket first (BFF pattern)
      const ticket = await getTicket();

      // Connect with ticket in query param
      const ws = new WebSocket(`${WS_BASE}/ws/audio?ticket=${ticket}`);
      wsRef.current = ws;

      ws.onopen = () => {
        setIsConnected(true);
        setIsConnecting(false);
        attemptRef.current = 0;
        logger.info('websocket_connected', { component: 'useAudioWebSocket' });

        // Start heartbeat
        heartbeatRef.current = setInterval(() => {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send('PING');
          }
        }, 30000);
      };

      ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data);
          if (message.type === 'transcription') {
            onTranscription(message.text, message.duration_seconds);
          }
        } catch (err) {
          logger.error('websocket_parse_error', err instanceof Error ? err : new Error(String(err)), {
            component: 'useAudioWebSocket',
          });
        }
      };

      ws.onclose = (event) => {
        setIsConnected(false);
        if (heartbeatRef.current) {
          clearInterval(heartbeatRef.current);
          heartbeatRef.current = null;
        }

        // Auto-reconnect with backoff (unless clean close)
        if (!event.wasClean && attemptRef.current < RECONNECT_DELAYS.length) {
          const delay = RECONNECT_DELAYS[attemptRef.current];
          logger.debug('websocket_reconnecting', {
            attempt: attemptRef.current,
            delay_ms: delay,
            component: 'useAudioWebSocket',
          });
          attemptRef.current += 1;
          setTimeout(() => connect(), delay);
        }
      };

      ws.onerror = () => {
        const err = new Error('WebSocket connection error');
        setError(err);
        onError?.(err);
        logger.error('websocket_error', err, { component: 'useAudioWebSocket' });
      };

    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err));
      setError(error);
      setIsConnecting(false);
      onError?.(error);
      logger.error('websocket_connect_failed', error, { component: 'useAudioWebSocket' });
    }
  }, [onTranscription, onError]);

  const disconnect = useCallback(() => {
    if (heartbeatRef.current) {
      clearInterval(heartbeatRef.current);
      heartbeatRef.current = null;
    }
    wsRef.current?.close(1000, 'User disconnect');
    wsRef.current = null;
    setIsConnected(false);
    logger.debug('websocket_disconnected', { component: 'useAudioWebSocket' });
  }, []);

  const sendAudio = useCallback((chunk: ArrayBuffer) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(chunk);
    }
  }, []);

  const endAudio = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send('END');
    }
  }, []);

  return {
    isConnected,
    isConnecting,
    error,
    connect,
    disconnect,
    sendAudio,
    endAudio,
  };
}
```

### 2.5.1 Frontend Sherpa-onnx WASM KWS

**Sherpa-onnx KWS** (Keyword Spotting) detecte les mots-cles directement dans le browser via WASM.

#### Ressources Officielles (VALIDEES)

| Ressource | URL |
|-----------|-----|
| **WASM CDN** | `https://cdn.jsdelivr.net/npm/sherpa-onnx-wasm@latest/index.js` |
| **Modele GigaSpeech** | `https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2` |

**Contenu du modele** :
- `encoder-epoch-99-avg-1.onnx`
- `decoder-epoch-99-avg-1.onnx`
- `joiner-epoch-99-avg-1.onnx`
- `tokens.txt`

```
┌─────────────────────────────────────────────────────────────────────┐
│                   Sherpa-onnx WASM KWS Integration                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. Charger WASM via CDN jsdelivr                                    │
│     - Script: sherpa-onnx-wasm@latest/index.js                       │
│                                                                      │
│  2. Charger modeles .onnx depuis public/                             │
│     - encoder, decoder, joiner, tokens.txt                           │
│                                                                      │
│  3. Configurer KeywordSpotter avec config exacte                     │
│     - keywordsFile: "./keywords.txt"                                 │
│     - keywordsThreshold: 0.25                                        │
│                                                                      │
│  4. AudioWorklet capture micro -> resample 16kHz mono                │
│                                                                      │
│  5. Callback onKeywordDetected -> ouvrir WebSocket /ws/audio         │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

#### Configuration JavaScript (EXACTE)

```javascript
// lib/audio/sherpaKws.ts - Configuration validee
const config = {
  featConfig: {
    sampleRate: 16000,
    featureDim: 80,
  },
  modelConfig: {
    transducer: {
      encoder: "./models/sherpa-onnx-kws-zipformer-gigaspeech/encoder-epoch-99-avg-1.onnx",
      decoder: "./models/sherpa-onnx-kws-zipformer-gigaspeech/decoder-epoch-99-avg-1.onnx",
      joiner: "./models/sherpa-onnx-kws-zipformer-gigaspeech/joiner-epoch-99-avg-1.onnx",
    },
    tokens: "./models/sherpa-onnx-kws-zipformer-gigaspeech/tokens.txt",
    numThreads: 2,
  },
  keywordsFile: "./models/keywords.txt",  // Contient "Hey Sherpa"
  keywordsScore: 1.0,
  keywordsThreshold: 0.25,  // Sensibilite (0.0-1.0)
};
```

#### Headers de Securite OBLIGATOIRES

**CRITIQUE** : Sans ces headers, le WASM et SharedArrayBuffer ne fonctionneront pas !

```typescript
// next.config.ts
const nextConfig = {
  async headers() {
    return [
      {
        source: '/(.*)',
        headers: [
          { key: 'Cross-Origin-Embedder-Policy', value: 'require-corp' },
          { key: 'Cross-Origin-Opener-Policy', value: 'same-origin' },
        ],
      },
    ];
  },
};
```

```python
# Backend FastAPI - middleware
@app.middleware("http")
async def add_security_headers(request: Request, call_next):
    response = await call_next(request)
    response.headers["Cross-Origin-Embedder-Policy"] = "require-corp"
    response.headers["Cross-Origin-Opener-Policy"] = "same-origin"
    return response
```

#### Keyword par Defaut

**Wake word initial** : `"Hey Sherpa"` (anglais, GigaSpeech compatible)

**Fichier `keywords.txt`** :
```
Hey Sherpa
```

**Note** : Le modele GigaSpeech est entraine sur l'anglais. Les keywords doivent etre en anglais.

### 2.5.2 Configuration Keyword Custom (Documentation)

**Avec Sherpa-onnx KWS, pas d'entrainement necessaire !**

Le systeme "Open Vocabulary" permet de changer le keyword via un simple fichier texte.

**Procedure pour configurer un nouveau keyword** :

```bash
# 1. Installer sherpa-onnx CLI (une seule fois)
pip install sherpa-onnx

# 2. Generer les tokens pour le keyword souhaite
# Exemple: "Hey Lia" -> tokens BPE
sherpa-onnx-cli text2token \
  --text "Hey Lia" \
  --tokens /path/to/tokens.txt \
  --output keywords.txt

# 3. Ajuster le fichier keywords.txt genere
# Format: tokens :boosting_score #trigger_threshold
# - boosting_score: 1.0-2.0 (aide le mot a survivre au beam search)
# - trigger_threshold: 0.0-1.0 (seuil de probabilite acoustique)

# Exemple de fichier keywords.txt final:
# ▁HEY ▁L I A :1.5 #0.35
# ▁OK ▁L I A :1.5 #0.40

# 4. Deployer le fichier dans le frontend
cp keywords.txt apps/web/public/models/keywords.txt
```

**Parametres de tuning** :

| Parametre | Valeur | Effet |
|-----------|--------|-------|
| `boosting_score` | 1.0-2.0 | Plus haut = moins de faux negatifs |
| `trigger_threshold` | 0.0-1.0 | Plus haut = moins de faux positifs |

**Recommandations** :
- Commencer avec `:1.5 #0.35` puis ajuster selon les tests
- Keywords courts (2-3 syllabes) fonctionnent mieux
- Eviter les mots communs ("Hello", "OK") sans prefixe distinctif

**Keywords suggeres** :
| Keyword | Tokens | Notes |
|---------|--------|-------|
| "Hey Lia" | `▁HEY ▁L I A` | **Recommande** - distinctif |
| "OK Lia" | `▁OK ▁L I A` | Alternative |
| "Lia" | `▁L I A` | Trop court, faux positifs |

### 2.5.3 Integration Next.js + Sherpa-onnx WASM

**Configuration Next.js 16** avec headers de securite et WASM :

```javascript
// next.config.ts
const nextConfig = {
  webpack: (config) => {
    // Support WASM async
    config.experiments = { ...config.experiments, asyncWebAssembly: true };
    return config;
  },

  // CRITIQUE: Headers pour SharedArrayBuffer (WASM multi-thread)
  async headers() {
    return [
      {
        source: '/(.*)',
        headers: [
          { key: 'Cross-Origin-Embedder-Policy', value: 'require-corp' },
          { key: 'Cross-Origin-Opener-Policy', value: 'same-origin' },
        ],
      },
    ];
  },
};
```

**Structure fichiers** :
```
apps/web/
├── public/
│   └── models/
│       ├── sherpa-onnx-kws-zipformer-gigaspeech/  # Modele decompresse
│       │   ├── encoder-epoch-99-avg-1.onnx
│       │   ├── decoder-epoch-99-avg-1.onnx
│       │   ├── joiner-epoch-99-avg-1.onnx
│       │   └── tokens.txt
│       │
│       └── keywords.txt           # "Hey Sherpa"
│
├── src/lib/audio/
│   ├── sherpaKws.ts               # Wrapper Sherpa-onnx KWS
│   ├── audioWorklet.ts            # AudioWorklet processor
│   └── recorder.ts                # Audio capture utilities
│
└── src/hooks/
    └── useSherpaKws.ts            # Hook React pour KWS
```

**Script de telechargement modele** :
```bash
#!/bin/bash
# scripts/download-kws-model.sh

MODEL_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2"
MODEL_DIR="apps/web/public/models"

mkdir -p $MODEL_DIR
cd $MODEL_DIR

# Telecharger et decompresser
wget -O kws-model.tar.bz2 $MODEL_URL
tar -xjf kws-model.tar.bz2
rm kws-model.tar.bz2

# Creer keywords.txt
echo "Hey Sherpa" > keywords.txt

echo "Modele KWS installe dans $MODEL_DIR"
```

**Initialisation Sherpa-onnx KWS (GOLD GRADE - patterns codebase)** :

```typescript
// lib/audio/sherpaKws.ts
/**
 * Sherpa-onnx WASM Keyword Spotting wrapper.
 *
 * Provides offline wake word detection in the browser.
 * Uses CDN for WASM module, local models from /public.
 */

// Types for Sherpa-onnx WASM API
export interface SherpaKwsConfig {
  featConfig: { sampleRate: number; featureDim: number };
  modelConfig: {
    transducer: { encoder: string; decoder: string; joiner: string };
    tokens: string;
    numThreads: number;
  };
  keywordsFile: string;
  keywordsScore: number;
  keywordsThreshold: number;
}

export interface SherpaKwsStream {
  acceptWaveform: (sampleRate: number, samples: Float32Array) => void;
}

export interface SherpaKwsResult {
  keyword: string | null;
}

export interface SherpaKwsInstance {
  createStream: () => SherpaKwsStream;
  isReady: (stream: SherpaKwsStream) => boolean;
  decode: (stream: SherpaKwsStream) => void;
  getResult: (stream: SherpaKwsStream) => SherpaKwsResult;
  reset: (stream: SherpaKwsStream) => void;
}

// CDN URL for WASM module
const WASM_CDN = 'https://cdn.jsdelivr.net/npm/sherpa-onnx-wasm@latest/index.js';

// Default configuration
const DEFAULT_CONFIG: SherpaKwsConfig = {
  featConfig: {
    sampleRate: 16000,
    featureDim: 80,
  },
  modelConfig: {
    transducer: {
      encoder: '/models/sherpa-onnx-kws-zipformer-gigaspeech/encoder-epoch-99-avg-1.onnx',
      decoder: '/models/sherpa-onnx-kws-zipformer-gigaspeech/decoder-epoch-99-avg-1.onnx',
      joiner: '/models/sherpa-onnx-kws-zipformer-gigaspeech/joiner-epoch-99-avg-1.onnx',
    },
    tokens: '/models/sherpa-onnx-kws-zipformer-gigaspeech/tokens.txt',
    numThreads: 2,
  },
  keywordsFile: '/models/keywords.txt',
  keywordsScore: 1.0,
  keywordsThreshold: 0.25,
};

/**
 * Initialize Sherpa-onnx KWS instance.
 *
 * @param config - Optional custom configuration
 * @returns Promise resolving to KWS instance
 * @throws Error if WASM loading or initialization fails
 */
export async function initSherpaKws(
  config: SherpaKwsConfig = DEFAULT_CONFIG
): Promise<SherpaKwsInstance> {
  // Dynamic import from CDN (webpackIgnore prevents bundling)
  const sherpaModule = await import(/* webpackIgnore: true */ WASM_CDN);
  const kws = await sherpaModule.createKws(config);
  return kws;
}
```

```typescript
// hooks/useSherpaKws.ts
'use client';

/**
 * useSherpaKws - Hook for Sherpa-onnx keyword spotting.
 *
 * Manages KWS lifecycle and provides:
 * - Automatic initialization on mount
 * - Audio processing for wake word detection
 * - Cleanup on unmount
 * - Error state for UI feedback
 *
 * Usage:
 * ```tsx
 * const { isReady, isLoading, error, processAudio } = useSherpaKws({
 *   onKeywordDetected: () => startRecording(),
 * });
 * ```
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { logger } from '@/lib/logger';
import {
  initSherpaKws,
  type SherpaKwsInstance,
  type SherpaKwsStream,
} from '@/lib/audio/sherpaKws';

export interface UseSherpaKwsOptions {
  /** Callback when wake word is detected */
  onKeywordDetected: () => void;
  /** Enable/disable KWS (default: true) */
  enabled?: boolean;
}

export interface UseSherpaKwsReturn {
  /** KWS is initialized and ready */
  isReady: boolean;
  /** KWS is initializing */
  isLoading: boolean;
  /** Initialization error (if any) */
  error: Error | null;
  /** Process audio samples for keyword detection */
  processAudio: (samples: Float32Array) => void;
}

export function useSherpaKws({
  onKeywordDetected,
  enabled = true,
}: UseSherpaKwsOptions): UseSherpaKwsReturn {
  const kwsRef = useRef<SherpaKwsInstance | null>(null);
  const streamRef = useRef<SherpaKwsStream | null>(null);

  const [isReady, setIsReady] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  // Initialize KWS on mount (when enabled)
  useEffect(() => {
    if (!enabled) {
      return;
    }

    let isMounted = true;
    setIsLoading(true);
    setError(null);

    initSherpaKws()
      .then((kws) => {
        if (!isMounted) return;

        kwsRef.current = kws;
        streamRef.current = kws.createStream();
        setIsReady(true);
        setIsLoading(false);

        logger.info('sherpa_kws_initialized', { component: 'useSherpaKws' });
      })
      .catch((err) => {
        if (!isMounted) return;

        const error = err instanceof Error ? err : new Error(String(err));
        setError(error);
        setIsLoading(false);

        logger.error('sherpa_kws_init_failed', error, { component: 'useSherpaKws' });
      });

    // Cleanup on unmount or when disabled
    return () => {
      isMounted = false;
      kwsRef.current = null;
      streamRef.current = null;
      setIsReady(false);
      logger.debug('sherpa_kws_cleanup', { component: 'useSherpaKws' });
    };
  }, [enabled]);

  // Process audio samples
  const processAudio = useCallback(
    (samples: Float32Array) => {
      const kws = kwsRef.current;
      const stream = streamRef.current;

      if (!kws || !stream) {
        return;
      }

      try {
        // Feed samples to stream (16kHz, Float32 [-1, 1])
        stream.acceptWaveform(16000, samples);

        // Decode while ready
        while (kws.isReady(stream)) {
          kws.decode(stream);
        }

        // Check for keyword
        const result = kws.getResult(stream);
        if (result.keyword) {
          logger.info('sherpa_kws_keyword_detected', {
            keyword: result.keyword,
            component: 'useSherpaKws',
          });

          kws.reset(stream); // Reset for next detection
          onKeywordDetected();
        }
      } catch (err) {
        logger.error('sherpa_kws_process_error', err instanceof Error ? err : new Error(String(err)), {
          component: 'useSherpaKws',
        });
      }
    },
    [onKeywordDetected]
  );

  return { isReady, isLoading, error, processAudio };
}
```
```

### 2.5.4 Metriques Prometheus STT

```python
# domains/voice/metrics.py
from prometheus_client import Counter, Histogram

stt_transcriptions_total = Counter(
    "stt_transcriptions_total",
    "Total STT transcriptions",
    ["status", "language"]
)

stt_transcription_duration_seconds = Histogram(
    "stt_transcription_duration_seconds",
    "STT transcription duration",
    buckets=[0.1, 0.5, 1.0, 2.0, 5.0, 10.0]
)

stt_audio_duration_seconds = Histogram(
    "stt_audio_duration_seconds",
    "Audio duration received",
    buckets=[1, 5, 10, 30, 60]
)

websocket_connections_active = Gauge(
    "voice_websocket_connections_active",
    "Active WebSocket connections"
)
```

### 2.5.5 Pattern Conformity - STT vs TTS

**Decision** : STT n'utilise PAS le pattern protocol/factory car :

| Aspect | TTS | STT |
|--------|-----|-----|
| **Providers multiples** | Oui (Edge, OpenAI) | Non (Sherpa-onnx uniquement) |
| **Switching runtime** | Oui (admin toggle) | Non necessaire |
| **API type** | REST endpoint | WebSocket streaming |

**Justification** : Le pattern factory est utile quand on a plusieurs providers interchangeables. Pour STT, on utilise uniquement Sherpa-onnx (gratuit, offline). Si un jour on ajoute Whisper/Deepgram comme fallback, on pourra refactorer.

**Architecture simplifiee** :
```
domains/voice/stt/
├── sherpa_stt.py       # Service unique (pas de protocol/factory)
├── audio_utils.py      # Utilitaires audio
└── schemas.py          # Pydantic schemas
```

### 2.6 Environnements Supportes

| Plateforme | Navigateur | Sherpa WASM | AudioWorklet | WebSocket | Notes |
|------------|------------|-------------|--------------|-----------|-------|
| **Android** | Chrome | Oui | Oui | Oui | Full support |
| **iOS** | Safari | Oui | Oui | Oui | Depuis Safari 14.1 |
| **Windows** | Chrome | Oui | Oui | Oui | Full support |
| **Windows** | Edge | Oui | Oui | Oui | Base Chromium |
| **macOS** | Chrome | Oui | Oui | Oui | Full support |
| **macOS** | Safari | Oui | Oui | Oui | Depuis Safari 14.1 |
| **Linux** | Firefox | Oui | Oui | Oui | Full support |

**Backend Sherpa-onnx (Python)** :
| Plateforme | Architecture | Support | Notes |
|------------|--------------|---------|-------|
| Windows | AMD64 | Oui | Wheels precompiles (DEV) |
| Raspberry Pi | ARM64 (aarch64) | Oui | Wheels precompiles (PROD) |
| Linux | AMD64 | Oui | Wheels precompiles |

**Frontend Sherpa-onnx (WASM)** :
| Feature | Support | Notes |
|---------|---------|-------|
| WASM SIMD | Recommande | Performance optimale |
| WASM non-SIMD | Fallback | ~2x plus lent |
| SharedArrayBuffer | Optionnel | Multi-threading |

### 2.7 Composants Frontend

```
apps/web/src/
├── hooks/
│   ├── useVoiceMode.ts           # Orchestration complete (state machine)
│   ├── useSherpaKws.ts           # Sherpa-onnx KWS WASM detection
│   ├── useAudioWebSocket.ts      # WebSocket audio streaming
│   ├── useAudioRecorder.ts       # WebAudio + AudioWorklet
│   ├── useVAD.ts                 # Voice Activity Detection
│   └── useVoiceSettings.ts       # Lecture/ecriture preferences
│
├── components/
│   ├── chat/
│   │   └── VoiceModeBadge.tsx    # Badge vert/gris cliquable
│   └── voice/
│       ├── VoiceOverlay.tsx      # Overlay plein ecran mode vocal actif
│       ├── ListeningIndicator.tsx # Animation "J'ecoute..."
│       ├── SpeakingIndicator.tsx  # Animation "Je parle..."
│       └── VoiceSettings.tsx      # Panel parametres vocaux
│
├── lib/
│   └── audio/
│       ├── sherpaKws.ts          # Sherpa-onnx KWS WASM wrapper
│       ├── audioWorklet.ts       # AudioWorklet processor
│       ├── recorder.ts           # Audio capture utilities
│       └── vad.ts                # Energy-based VAD
│
└── stores/
    └── voiceModeStore.ts         # Zustand store pour etat global
```

**Dependances npm** :
- Aucune dependance npm specifique (Sherpa-onnx WASM charge depuis public/)
- Configuration Webpack pour WASM async

### 2.8 Backend STT (Sherpa-onnx)

```
apps/api/src/domains/voice/
├── __init__.py
├── stt/
│   ├── __init__.py
│   ├── sherpa_stt.py            # SherpaSttService (OfflineRecognizer)
│   ├── audio_utils.py           # Conversion audio (resample, format)
│   └── schemas.py               # Pydantic schemas
├── router.py                    # WebSocket /ws/audio + REST endpoints
├── service.py                   # VoiceService orchestration
├── websocket_manager.py         # Gestion connexions WebSocket
└── schemas.py                   # VoiceSettings schema
```

**Endpoints** :
```python
# WebSocket /ws/audio
# Protocol:
# - Client envoie chunks audio (PCM 16kHz int16)
# - Client envoie "END" quand fin de phrase
# - Server repond avec transcription JSON

# GET/PATCH /api/v1/auth/me/voice-settings
# Gestion des preferences vocales utilisateur
```

**Modele STT** :
```
/models/sensevoice/
├── model.onnx                   # Modele ONNX (~100MB)
├── tokens.txt                   # Vocabulaire
└── README.md                    # Documentation modele
```

### 2.9 Fichiers a Creer/Modifier

**Backend** :
| Fichier | Action | Description |
|---------|--------|-------------|
| `domains/voice/stt/__init__.py` | Creer | Package STT |
| `domains/voice/stt/sherpa_stt.py` | Creer | SherpaSttService (OfflineRecognizer) |
| `domains/voice/stt/audio_utils.py` | Creer | Conversion audio (resample, format) |
| `domains/voice/stt/schemas.py` | Creer | TranscriptionRequest/Response |
| `domains/voice/router.py` | Modifier | Ajouter WebSocket /ws/audio + /voice-settings |
| `domains/voice/websocket_manager.py` | Creer | Gestion connexions WebSocket |
| `domains/voice/service.py` | Modifier | Ajouter transcription logic |
| `core/config/voice.py` | Modifier | Ajouter config STT (model path) |
| `domains/auth/models.py` | Modifier | Ajouter VoiceSettings a User |
| `Dockerfile` | Modifier | Telecharger modele SenseVoiceSmall |
| `requirements.txt` | Modifier | Ajouter sherpa-onnx, numpy |

**Frontend** :
| Fichier | Action | Description |
|---------|--------|-------------|
| `hooks/useVoiceMode.ts` | Creer | State machine orchestration |
| `hooks/useSherpaKws.ts` | Creer | Sherpa-onnx KWS WASM detection |
| `hooks/useAudioWebSocket.ts` | Creer | WebSocket audio streaming |
| `hooks/useAudioRecorder.ts` | Creer | WebAudio + AudioWorklet |
| `hooks/useVAD.ts` | Creer | Voice Activity Detection |
| `hooks/useVoiceSettings.ts` | Creer | API settings hook |
| `components/chat/VoiceModeBadge.tsx` | Creer | Badge toggle vert/gris |
| `components/chat/ChatMessage.tsx` | Modifier | Indicateur source vocale |
| `components/voice/VoiceOverlay.tsx` | Creer | Overlay mode vocal |
| `components/voice/ListeningIndicator.tsx` | Creer | Animation ecoute |
| `components/voice/VoiceSettings.tsx` | Creer | Panel settings |
| `lib/audio/sherpaKws.ts` | Creer | Sherpa-onnx KWS WASM wrapper |
| `lib/audio/audioWorklet.ts` | Creer | AudioWorklet processor |
| `lib/audio/recorder.ts` | Creer | Audio utilities |
| `lib/audio/vad.ts` | Creer | Energy-based VAD |
| `stores/voiceModeStore.ts` | Creer | Zustand state |
| `types/chat.ts` | Modifier | Ajouter source, audioDurationSeconds |
| `locales/*/translation.json` | Modifier | i18n pour voice UI |
| `public/models/sherpa-onnx-kws/` | Creer | Fichiers WASM + modele KWS |
| `public/models/sherpa-onnx-kws/keywords.txt` | Creer | Mots-cles configures |
| `next.config.ts` | Modifier | Support WASM async |

---

## 3. Hybrid Memory Search

### 3.1 Concept

Combiner **recherche semantique** (embeddings) et **recherche keyword** (BM25) :

```
Query: "anniversaire de Marie"

Semantic Search (actuel):
- Trouve: "fete de ma femme en juin" (similar meaning)
- Score: 0.82

BM25 Keyword Search (nouveau):
- Trouve: "anniversaire Marie le 15 juin" (exact words)
- Score: 0.95

Hybrid Score:
- final_score = α × semantic + (1-α) × bm25
- α = 0.6 (configurable)
- Result: priorite aux resultats qui matchent les deux
```

### 3.2 Architecture avec Cache (Performance Optimisee)

**Probleme initial** : Charger toutes les memoires a chaque requete = O(n) * O(requetes)

**Solution** : Index BM25 pre-calcule et cache par utilisateur

```
┌─────────────────────────────────────────────────────────────────┐
│                    BM25 INDEX LIFECYCLE                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  CREATION/UPDATE :                                               │
│  ┌─────────────────┐                                             │
│  │ Memory Created/ │ → Invalidate cache user:{user_id}:bm25     │
│  │ Updated/Deleted │                                             │
│  └─────────────────┘                                             │
│                                                                  │
│  SEARCH :                                                        │
│  ┌─────────────────┐    Cache hit?     ┌─────────────────┐      │
│  │ Hybrid Search   │ ───────────────── │ Use cached BM25 │      │
│  │ Request         │        │          │ Index           │      │
│  └─────────────────┘        │ No       └─────────────────┘      │
│                             ▼                                    │
│                    ┌─────────────────┐                           │
│                    │ Build BM25 Index│                           │
│                    │ + Cache (5min)  │                           │
│                    └─────────────────┘                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 3.3 Implementation Detaillee (GOLD GRADE)

> **Corrections appliquees** : DRY, patterns codebase, logging, metrics, exceptions, singleton

#### 3.3.1 Exceptions (core/exceptions.py - AJOUT)

```python
# Ajouter dans core/exceptions.py

class HybridSearchError(BaseAPIException):
    """Error during hybrid memory search."""
    def __init__(
        self,
        detail: str = "Hybrid search failed",
        **log_context: Any,
    ) -> None:
        super().__init__(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=detail,
            log_level="error",
            log_event="hybrid_search_error",
            **log_context,
        )
```

#### 3.3.2 Configuration (core/config/agents.py - AJOUT)

```python
# Ajouter dans AgentsSettings class

# === HYBRID MEMORY SEARCH ===
memory_hybrid_enabled: bool = Field(
    default=True,
    description="Enable hybrid BM25+semantic search for memories",
)
memory_hybrid_alpha: float = Field(
    default=0.6,
    ge=0.0,
    le=1.0,
    description="Weight for semantic score (1-alpha for BM25)",
)
memory_hybrid_min_score: float = Field(
    default=0.5,
    ge=0.0,
    le=1.0,
    description="Minimum combined score for inclusion",
)
memory_hybrid_boost_threshold: float = Field(
    default=0.5,
    ge=0.0,
    le=1.0,
    description="Threshold for both-high bonus",
)
memory_bm25_cache_max_users: int = Field(
    default=100,
    ge=10,
    le=1000,
    description="Max users in BM25 local cache",
)
```

#### 3.3.3 Metrics (infrastructure/observability/metrics.py - AJOUT)

```python
# Ajouter dans metrics.py

from prometheus_client import Counter, Histogram, Gauge

# Hybrid Memory Search Metrics
hybrid_search_total = Counter(
    "memory_hybrid_search_total",
    "Total hybrid search operations",
    ["status"],  # success, error, fallback
)

hybrid_search_duration_seconds = Histogram(
    "memory_hybrid_search_duration_seconds",
    "Hybrid search latency",
    buckets=[0.01, 0.05, 0.1, 0.2, 0.5, 1.0],
)

bm25_cache_hits_total = Counter(
    "memory_bm25_cache_hits_total",
    "BM25 index cache hits",
)

bm25_cache_misses_total = Counter(
    "memory_bm25_cache_misses_total",
    "BM25 index cache misses",
)

bm25_cache_size = Gauge(
    "memory_bm25_cache_size",
    "Current BM25 cache size (users)",
)
```

#### 3.3.4 BM25 Index Manager (infrastructure/store/bm25_index.py - CREER)

```python
"""
BM25 Index Manager for Hybrid Memory Search.

Provides cached BM25 indices per user with LRU eviction.
Follows codebase patterns: singleton, structured logging, metrics.
"""
import re
import hashlib
from functools import lru_cache
from typing import TYPE_CHECKING

from rank_bm25 import BM25Okapi

from src.core.config import get_settings
from src.infrastructure.observability.logging import get_logger
from src.infrastructure.observability.metrics import (
    bm25_cache_hits_total,
    bm25_cache_misses_total,
    bm25_cache_size,
)

if TYPE_CHECKING:
    from src.core.config import Settings

logger = get_logger(__name__)

# Regex pattern for French-aware tokenization (compile once)
_TOKEN_PATTERN = re.compile(r"[\w']+", re.UNICODE)


def tokenize_text(text: str) -> list[str]:
    """
    Tokenize text for BM25 scoring.

    French-aware: keeps accents via UNICODE flag.
    Filters tokens shorter than 2 chars (noise).

    Args:
        text: Input text to tokenize

    Returns:
        List of lowercase tokens
    """
    tokens = _TOKEN_PATTERN.findall(text.lower())
    return [t for t in tokens if len(t) > 1]


class BM25IndexManager:
    """
    Manages BM25 indices with per-user caching.

    Thread-safe for read operations. Cache uses content hash
    for automatic invalidation on corpus changes.

    Usage:
        manager = get_bm25_manager()
        bm25, doc_ids = manager.get_or_build_index(user_id, docs, doc_ids)
        scores = bm25.get_scores(tokenize_text(query))
    """

    def __init__(self, settings: "Settings") -> None:
        self._local_cache: dict[str, tuple[BM25Okapi, list[str]]] = {}
        self._max_users = settings.memory_bm25_cache_max_users
        logger.info(
            "bm25_manager_initialized",
            max_users=self._max_users,
        )

    def get_or_build_index(
        self,
        user_id: str,
        documents: list[str],
        document_ids: list[str],
    ) -> tuple[BM25Okapi, list[str]]:
        """
        Get cached BM25 index or build new one.

        Args:
            user_id: User ID for cache scoping
            documents: List of document contents
            document_ids: List of document IDs (for result mapping)

        Returns:
            Tuple of (BM25Okapi instance, document_ids)
        """
        content_hash = self._compute_hash(documents)
        cache_key = f"bm25:{user_id}:{content_hash}"

        # Cache hit
        if cache_key in self._local_cache:
            bm25_cache_hits_total.inc()
            logger.debug(
                "bm25_cache_hit",
                user_id=user_id,
                cache_key=cache_key,
            )
            return self._local_cache[cache_key]

        # Cache miss - build index
        bm25_cache_misses_total.inc()
        tokenized = [tokenize_text(doc) for doc in documents]
        bm25 = BM25Okapi(tokenized)

        # LRU eviction
        if len(self._local_cache) >= self._max_users:
            evicted_key = next(iter(self._local_cache))
            del self._local_cache[evicted_key]
            logger.debug("bm25_cache_eviction", evicted_key=evicted_key)

        self._local_cache[cache_key] = (bm25, document_ids)
        bm25_cache_size.set(len(self._local_cache))

        logger.debug(
            "bm25_index_built",
            user_id=user_id,
            document_count=len(documents),
            cache_size=len(self._local_cache),
        )

        return bm25, document_ids

    def _compute_hash(self, documents: list[str]) -> str:
        """Compute content hash for cache invalidation."""
        content = "".join(sorted(documents))
        return hashlib.md5(content.encode()).hexdigest()[:8]

    def invalidate_user_cache(self, user_id: str) -> None:
        """Invalidate all BM25 caches for a user."""
        keys_to_remove = [
            k for k in self._local_cache
            if k.startswith(f"bm25:{user_id}:")
        ]
        for k in keys_to_remove:
            del self._local_cache[k]

        if keys_to_remove:
            bm25_cache_size.set(len(self._local_cache))
            logger.info(
                "bm25_cache_invalidated",
                user_id=user_id,
                keys_removed=len(keys_to_remove),
            )


@lru_cache
def get_bm25_manager() -> BM25IndexManager:
    """Get singleton BM25IndexManager instance."""
    return BM25IndexManager(get_settings())
```

#### 3.3.5 Hybrid Search Function (infrastructure/store/semantic_store.py - AJOUT)

```python
# Ajouter dans semantic_store.py

from src.core.config import get_settings
from src.infrastructure.store.bm25_index import (
    BM25IndexManager,
    get_bm25_manager,
    tokenize_text,
)
from src.infrastructure.observability.metrics import (
    hybrid_search_total,
    hybrid_search_duration_seconds,
)

async def search_hybrid(
    store: BaseStore,
    namespace: StoreNamespace,
    query: str,
    limit: int = 10,
    min_score: float | None = None,
    alpha: float | None = None,
    bm25_manager: BM25IndexManager | None = None,
) -> list[SearchItem]:
    """
    Hybrid search combining semantic (pgvector) and BM25 scoring.

    Uses settings defaults if min_score/alpha not provided.
    Automatically uses singleton BM25IndexManager if not injected.

    Args:
        store: LangGraph store with pgvector
        namespace: Target namespace for search
        query: Natural language query
        limit: Maximum results to return
        min_score: Minimum combined score (default from settings)
        alpha: Semantic weight, 1-alpha for BM25 (default from settings)
        bm25_manager: Optional custom BM25 manager (for testing)

    Returns:
        List of SearchItem sorted by hybrid score

    Raises:
        HybridSearchError: On search failure
    """
    settings = get_settings()
    min_score = min_score or settings.memory_hybrid_min_score
    alpha = alpha or settings.memory_hybrid_alpha
    boost_threshold = settings.memory_hybrid_boost_threshold

    # Use singleton if not injected
    if bm25_manager is None:
        bm25_manager = get_bm25_manager()

    with hybrid_search_duration_seconds.time():
        try:
            # 1. Semantic search (fast, indexed via pgvector)
            semantic_results = await search_semantic(
                store, namespace, query, limit=limit * 3, min_score=0.3
            )

            if not semantic_results:
                hybrid_search_total.labels(status="fallback").inc()
                logger.debug(
                    "hybrid_search_no_semantic_results",
                    namespace=namespace.to_tuple(),
                )
                return []

            # 2. Get all items for BM25 corpus
            all_items = await store.asearch(
                namespace.to_tuple(), query="", limit=500
            )

            if not all_items:
                hybrid_search_total.labels(status="fallback").inc()
                return list(semantic_results[:limit])

            # 3. Build/get BM25 index
            documents = [item.value.get("content", "") for item in all_items]
            document_ids = [item.key for item in all_items]

            bm25, _ = bm25_manager.get_or_build_index(
                namespace.user_id, documents, document_ids
            )

            # 4. Score query with BM25 (use same tokenizer!)
            query_tokens = tokenize_text(query)

            # Guard: empty query tokens → fallback to semantic only
            if not query_tokens:
                hybrid_search_total.labels(status="fallback").inc()
                logger.debug(
                    "hybrid_search_empty_query_tokens",
                    query=query,
                    namespace=namespace.to_tuple(),
                )
                return list(semantic_results[:limit])

            bm25_scores = bm25.get_scores(query_tokens)

            # Guard: empty or all-zero scores → avoid division by zero
            max_bm25_raw = max(bm25_scores) if bm25_scores else 0.0
            max_bm25 = max_bm25_raw if max_bm25_raw > 0 else 1.0

            # 5. Create lookup maps
            semantic_scores = {r.key: r.score for r in semantic_results}
            bm25_score_map = {
                document_ids[i]: bm25_scores[i] / max_bm25
                for i in range(len(document_ids))
            }

            # 6. Combine scores
            combined: list[SearchItem] = []
            seen_keys: set[str] = set()

            for item in all_items:
                key = item.key
                if key in seen_keys:
                    continue
                seen_keys.add(key)

                sem_score = semantic_scores.get(key, 0.0)
                bm25_score = bm25_score_map.get(key, 0.0)

                # Hybrid scoring formula
                final_score = alpha * sem_score + (1 - alpha) * bm25_score

                # Boost if both scores are high
                if sem_score > boost_threshold and bm25_score > boost_threshold:
                    final_score *= 1.1

                if final_score >= min_score:
                    combined.append(SearchItem(
                        namespace=item.namespace,
                        key=item.key,
                        value=item.value,
                        created_at=item.created_at,
                        updated_at=item.updated_at,
                        score=final_score,
                    ))

            # 7. Sort and limit
            combined.sort(key=lambda x: x.score or 0.0, reverse=True)

            hybrid_search_total.labels(status="success").inc()
            logger.debug(
                "hybrid_search_completed",
                namespace=namespace.to_tuple(),
                semantic_count=len(semantic_results),
                bm25_corpus_size=len(all_items),
                result_count=len(combined[:limit]),
            )

            return combined[:limit]

        except Exception as e:
            hybrid_search_total.labels(status="error").inc()
            logger.error(
                "hybrid_search_failed",
                namespace=namespace.to_tuple(),
                error=str(e),
            )
            # Fallback to semantic only
            return list(semantic_results[:limit]) if semantic_results else []
```

### 3.4 Fichiers a Modifier

| Fichier | Action | Description |
|---------|--------|-------------|
| `infrastructure/store/bm25_index.py` | Creer | BM25IndexManager avec cache |
| `infrastructure/store/semantic_store.py` | Modifier | Ajouter `search_hybrid()` |
| `domains/agents/middleware/memory_injection.py` | Modifier | Utiliser hybrid search |
| `core/config/settings.py` | Modifier | Ajouter `MEMORY_HYBRID_*` config |
| `main.py` | Modifier | Initialiser BM25IndexManager singleton |

### 3.5 Configuration

```bash
# .env
MEMORY_HYBRID_ENABLED=true
MEMORY_HYBRID_ALPHA=0.6           # Poids semantic vs BM25
MEMORY_HYBRID_MIN_SCORE=0.5       # Score minimum pour inclusion
MEMORY_HYBRID_BOOST_THRESHOLD=0.5 # Seuil pour bonus reinforcement
```

## 4. Ordre d'Implementation Recommande

> **PRIORITE** : Hybrid Memory Search d'abord (isole, moins risque), puis Voice System.

---

### Phase 1 : Hybrid Memory Search (~2 jours)

**Objectif** : Ameliorer la recherche memoire avec BM25 + Semantic scoring.

1. Creer `infrastructure/store/bm25_index.py` (BM25IndexManager avec cache)
2. Ajouter `search_hybrid()` dans `infrastructure/store/semantic_store.py`
3. Modifier `domains/agents/middleware/memory_injection.py` pour utiliser hybrid search
4. Ajouter config dans `.env` : `MEMORY_HYBRID_*`
5. Tests unitaires : keyword match, semantic match, cache hit/invalidation
6. Tests performance : <100ms pour 100 memories, <200ms pour 500

**Fichiers a creer/modifier** :
```
infrastructure/store/bm25_index.py         # CREER - BM25IndexManager
infrastructure/store/semantic_store.py     # MODIFIER - search_hybrid()
domains/agents/middleware/memory_injection.py  # MODIFIER - utiliser hybrid
core/config/settings.py                    # MODIFIER - MEMORY_HYBRID_* config
requirements.txt                           # MODIFIER - rank-bm25>=0.2.2
```

**Delivrable** : Recherche memoire amelioree, feature isolee et testee

---

### Phase 2 : Backend Sherpa-onnx STT (~3-4 jours)
1. Modifier `Dockerfile` pour telecharger modele SenseVoiceSmall
2. Creer `domains/voice/stt/sherpa_stt.py` (SherpaSttService)
3. Creer wrapper async `transcribe_async()` avec `run_in_executor` (section 8.3)
4. Creer `domains/voice/websocket_manager.py` (gestion WS)
5. Ajouter WebSocket `/ws/audio` dans `router.py` avec :
   - Authentification JWT via query param (section 8.2)
   - Rate limiting (10 connexions/min/user)
6. Ajouter VoiceSettings au model User
7. Tests unitaires STT + Docker multi-arch (AMD64/ARM64)

**Delivrable** : WebSocket `/ws/audio` fonctionnel avec Sherpa-onnx, auth JWT, rate limiting

### Phase 3 : Frontend Sherpa-onnx WASM KWS
1. **PREREQUIS CRITIQUE** : Migrer OAuth popup → redirect flow (voir section 8.1)
2. Executer `scripts/download-kws-model.sh` pour telecharger modele GigaSpeech
3. Configurer Next.js avec headers COOP/COEP (SharedArrayBuffer)
4. Configurer backend FastAPI avec headers COOP/COEP
5. **Tester OAuth redirect flow** avec COOP/COEP actifs
6. Deployer modeles .onnx dans `public/models/sherpa-onnx-kws-zipformer-gigaspeech/`
7. Creer `keywords.txt` avec "Hey Sherpa"
8. Creer `lib/audio/sherpaKws.ts` (wrapper avec CDN jsdelivr)
9. Creer `useSherpaKws.ts` (hook detection avec API validee)
10. Creer `useAudioRecorder.ts` (AudioWorklet 16kHz)
11. Creer `useVAD.ts` (energy-based silence detection)
12. Tests manuels cross-browser (Chrome, Safari, Edge, Firefox)

**Delivrable** : Detection keyword "Hey Sherpa" fonctionnelle via Sherpa-onnx WASM CDN + OAuth fonctionnel

### Phase 4 : Badge Vocal + Talk Mode (~3-4 jours)
1. Creer `VoiceModeBadge.tsx` avec toggle vert/gris
2. Creer `VoiceOverlay.tsx` pour mode vocal actif
3. Creer `useAudioWebSocket.ts` (streaming WebSocket)
4. Creer `useVoiceMode.ts` (state machine orchestration)
5. Integrer WebSocket avec STT backend
6. Ajouter indicateur vocal dans `ChatMessage.tsx`
7. i18n pour tous les textes vocaux

**Delivrable** : Mode vocal complet avec cycle wake → speak → transcribe → TTS → listen

### Phase 5 : Polish & Settings (~1-2 jours)
1. Creer `VoiceSettings.tsx` dans parametres
2. Endpoint `/voice-settings` complet
3. Tests E2E complets
4. Documentation utilisateur

**Delivrable** : Feature complete et documentee

---

## 5. Configuration Complete

```bash
# .env additions

# === SHERPA-ONNX STT ===
SHERPA_MODEL_PATH=/models/sensevoice     # Path vers modele SenseVoiceSmall
SHERPA_NUM_THREADS=4                     # Threads CPU (ajuster selon hardware)
SHERPA_USE_ITN=true                      # Inverse Text Normalization
VOICE_STT_MAX_DURATION_SECONDS=60        # Max audio duration

# === VOICE MODE ===
# (Preferences utilisateur stockees en DB)
# Default keyword: "Hey Sherpa" (anglais, GigaSpeech model)
# Default VAD silence threshold: 1000ms
# Note: TTS auto-play gere par systeme TTS existant

# === SHERPA-ONNX KWS (Frontend WASM) ===
# Configuration dans public/models/sherpa-onnx-kws/keywords.txt
# Format: tokens :boosting_score #trigger_threshold
# Exemple: ▁HEY ▁L I A :1.5 #0.35

# === HYBRID MEMORY SEARCH ===
MEMORY_HYBRID_ENABLED=true
MEMORY_HYBRID_ALPHA=0.6                  # Poids semantic (1-α = BM25)
MEMORY_HYBRID_MIN_SCORE=0.5              # Score minimum inclusion
MEMORY_HYBRID_BOOST_THRESHOLD=0.5        # Seuil bonus reinforcement
MEMORY_BM25_CACHE_MAX_USERS=100          # Max users en cache local
```

---

## 6. Criteres de Verification (Definition of Done)

### Voice System - Tests Fonctionnels

| Test | Critere |
|------|---------|
| **STT Transcription** | Audio 5s FR transcrit correctement (>90% accuracy) |
| **STT Multi-langue** | FR, EN, DE, ES, IT, ZH transcrits (modele SenseVoiceSmall) |
| **STT Offline** | Transcription fonctionne sans internet |
| **WebSocket Connect** | Connexion /ws/audio etablie en <500ms |
| **Audio Streaming** | Chunks PCM 16kHz transmis correctement |
| **Audio Recording** | Chrome, Safari, Edge, Firefox supportes |
| **Keyword Detection** | "Hey Sherpa" detecte en <200ms (Sherpa-onnx KWS WASM CDN) |
| **Keyword Local** | Detection 100% dans le browser (pas de cloud) |
| **Keyword False Positive** | Pas de declenchement sur bruit/autre parole |
| **VAD Silence** | Fin de phrase detectee apres 1s silence |
| **Talk Mode Loop** | Cycle complet: wake → speak → transcribe → LLM → TTS → listen |
| **TTS Interruption** | Stop TTS si user parle pendant lecture |
| **Badge Toggle** | Vert/gris bascule correctement |
| **Mode Texte Masque** | Champ texte disparait en mode vocal |
| **Error Handling** | Toast si micro refuse, timeout, erreur reseau |
| **Docker Multi-arch** | Build AMD64 (dev) + ARM64 (Pi) fonctionnels |

### Voice System - Tests Non-Fonctionnels

| Test | Critere |
|------|---------|
| **Latence E2E** | < 3s de keyword a debut TTS (Sherpa KWS + STT) |
| **Memory Usage Browser** | Sherpa-onnx KWS WASM < 30MB RAM |
| **Memory Usage Backend** | Sherpa-onnx + modele < 500MB RAM |
| **CPU Usage Pi** | Transcription < 2s pour 10s audio sur Pi 4 |
| **Battery Mobile** | Mode ecoute < 2% batterie/heure (Sherpa KWS offline) |
| **Accessibility** | Keyboard navigation preserved, annonces ARIA |

### Hybrid Memory Search - Tests Fonctionnels

| Test | Critere |
|------|---------|
| **Keyword Match** | "anniversaire Marie" trouve memoire exacte |
| **Semantic Match** | "fete de ma femme" trouve memoire semantiquement proche |
| **Hybrid Boost** | Score keyword+semantic > score individuel |
| **Cache Performance** | 2eme requete < 50ms (cache hit) |
| **Cache Invalidation** | Ajout memoire → cache invalide |
| **Fallback** | Si BM25 echoue → retour semantic only |

### Hybrid Memory Search - Tests Non-Fonctionnels

| Test | Critere |
|------|---------|
| **Performance 100 memories** | < 100ms |
| **Performance 500 memories** | < 200ms |
| **Memory footprint** | Cache < 10MB par user |

---

## 7. Dependances et Prerequisites

### Backend
- `sherpa-onnx>=1.10.0` (STT offline, wheels AMD64/ARM64)
- `numpy>=1.24.0` (audio processing)
- `rank-bm25>=0.2.2` (pour BM25)

### Frontend
- Sherpa-onnx WASM KWS (build depuis source ou pre-built)
- Modele KWS zipformer-gigaspeech-3.3M (~3MB)
- Navigateurs avec WebAssembly + AudioWorklet + WebSocket: Chrome, Edge, Safari, Firefox
- HTTPS requis pour microphone access

### Infrastructure
- Modele SenseVoiceSmall (~100MB) telecharge dans Docker
- Pas de nouvelle infrastructure DB requise
- Pas de cle API externe (100% offline)

### Modele STT a telecharger
```
URL: https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.10.16/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2
Taille: ~100MB
Langues: FR, EN, DE, ES, IT, ZH, JA, KO
```

---

## 8. Risques et Mitigations

| Risque | Impact | Mitigation |
|--------|--------|------------|
| **OAuth popups casses par COOP/COEP** | **CRITIQUE** | Migrer vers redirect flow (section 8.1) |
| **Headers COOP/COEP cassent resources externes** | Moyen | Tester toutes les ressources (images, fonts, APIs) |
| **WebSocket sans auth/rate limit** | Moyen | JWT auth + rate limiting (section 8.2) |
| **STT bloquant event loop** | Moyen | run_in_executor (section 8.3) |
| Sherpa-onnx performance sur Pi | Faible | SenseVoiceSmall optimise CPU, tests benchmarks |
| WASM non supporte (vieux browsers) | Tres faible | Tous browsers modernes supportent WASM |
| Keyword false positives | Faible | keywordsThreshold ajustable (defaut 0.25) |
| Keyword en anglais uniquement | Faible | "Hey Sherpa" fonctionne bien avec GigaSpeech EN |
| CDN jsdelivr indisponible | Faible | Fallback: heberger WASM localement |
| BM25 lent sur gros corpus | Faible | Cache + limite 500 memoires |
| Micro refuse par user | Faible | UI claire + mode texte preserved |
| Modele STT trop gros pour Pi | Faible | SenseVoiceSmall = ~100MB, Pi 4 a 4-8GB RAM |
| WebSocket timeout | Faible | Heartbeat + reconnexion automatique |

**Note importante sur COOP/COEP** :
Les headers `Cross-Origin-Embedder-Policy: require-corp` et `Cross-Origin-Opener-Policy: same-origin`
sont requis pour SharedArrayBuffer (WASM multi-thread). Cela peut casser :
- Images/fonts depuis CDN sans CORS
- iframes externes
- OAuth popups (necessitent adaptation)

Tester minutieusement toutes les fonctionnalites existantes apres activation.

### 8.1 RISQUE CRITIQUE : OAuth Popups vs COOP/COEP

**Probleme** : `Cross-Origin-Opener-Policy: same-origin` bloque `window.opener` dans les popups OAuth.

**Impact** : Le flow Google OAuth actuel utilise des popups → **cassé** avec ces headers.

**Solutions possibles** :

| Solution | Complexite | Recommandation |
|----------|------------|----------------|
| **Redirect flow OAuth** | Moyenne | **RECOMMANDE** - Remplacer popup par redirect |
| Headers conditionnels | Haute | Appliquer COOP/COEP uniquement sur `/chat` |
| credentialless iframe | Haute | Experimentale, pas supportee partout |

**Implementation Redirect Flow** :
```typescript
// Au lieu de popup OAuth:
// window.open('/auth/google', 'oauth', 'popup')

// Utiliser redirect:
window.location.href = '/auth/google?redirect_uri=' + encodeURIComponent(window.location.href);
```

**Action requise Phase 2** : Migrer OAuth vers redirect flow AVANT d'activer COOP/COEP.

### 8.2 WebSocket Authentification (BFF Ticket System) et Rate Limiting

**IMPORTANT** : Le codebase utilise le BFF pattern avec cookies HTTP-only.
`verify_access_token()` n'existe pas. Solution: **Ticket System** (voir section 2.4.4).

**Flow Ticket System** :
```
1. Frontend appelle POST /api/v1/voice/ticket (avec cookie de session)
2. Backend génère ticket UUID, stocke en Redis (TTL 60s)
3. Frontend reçoit ticket, connecte WebSocket /ws/audio?ticket=xxx
4. Backend valide ticket, supprime (single-use), procède
```

**Authentification via Ticket** :
```python
# domains/voice/router.py
from fastapi import WebSocket, Query
from src.domains.voice.ticket_store import WebSocketTicketStore
from src.infrastructure.cache.redis import get_redis_session

@router.websocket("/ws/audio")
async def websocket_audio(
    websocket: WebSocket,
    ticket: str = Query(..., description="WebSocket authentication ticket"),
):
    """WebSocket with BFF ticket auth (not JWT)."""
    redis = await get_redis_session()
    ticket_store = WebSocketTicketStore(redis)

    user_id = await ticket_store.validate_and_consume_ticket(ticket)
    if not user_id:
        await websocket.close(code=4001, reason="Invalid or expired ticket")
        return

    await websocket.accept()
    # ... rest of handler
```

**Rate Limiting** (pattern from `auth/dependencies.py`) :
```python
from src.infrastructure.cache.redis import get_redis_cache
from src.infrastructure.rate_limiting.redis_limiter import RedisRateLimiter

# Max 10 connexions/minute par user
redis = await get_redis_cache()
limiter = RedisRateLimiter(redis)
rate_limit_key = f"ws:audio:{user_id}"

allowed = await limiter.acquire(
    key=rate_limit_key,
    max_calls=10,
    window_seconds=60,
)
if not allowed:
    await websocket.close(code=4029, reason="Rate limited")
    return
```

### 8.3 STT Non-Bloquant

**Probleme** : `stt_service.transcribe()` est synchrone → bloque l'event loop async.

**Solution** : Utiliser `run_in_executor` :
```python
import asyncio
from concurrent.futures import ThreadPoolExecutor

# Pool de threads pour STT (CPU-bound)
stt_executor = ThreadPoolExecutor(max_workers=4, thread_name_prefix="stt")

async def transcribe_async(audio_samples: list[float]) -> str:
    """Non-blocking STT transcription."""
    loop = asyncio.get_event_loop()
    return await loop.run_in_executor(
        stt_executor,
        stt_service.transcribe,
        audio_samples,
    )
```

