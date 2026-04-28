# Especificacions Tècniques: Rust Universal LLM Gateway (Hybrid)

## 1. Objectiu
Crear un proxy d'alt rendiment en **Rust** que centralitzi les crides a models de llenguatge. El sistema ha de ser capaç de redirigir peticions a backends locals (format OpenAI) o a APIs externes (Anthropic, Google Gemini) però això ho farem a la fase 2 (de moment no!), gestionant la traducció de protocols, l'autenticació d'usuaris i el registre de consum.

## 2. Stack Tecnològic
- **Llenguatge:** Rust (Latest Stable).
- **Framework Web:** `Axum` (asíncron).
- **Client HTTP:** `reqwest` amb suport per a `stream`.
- **Base de dades:** `SQLite` amb `SQLx`.
- **Traducció de format:** `Serde` per a la transformació de JSON.

## 3. Arquitectura de Base de Dades (Esquema Evolucionat)

### Taula `backends`
- `id`: INTEGER PRIMARY KEY
- `model_alias`: TEXT UNIQUE NOT NULL (Ex: "gpt-4-local", "claude-3-sonnet")
- `provider_type`: TEXT NOT NULL (Valors: `openai_compatible`, `anthropic`, `google_gemini`)
- `endpoint_url`: TEXT NOT NULL
- `model_name`: TEXT NOT NULL (El nom del model segons el proveïdor)
- `api_key`: TEXT (Opcional, per a serveis externs)
- `active`: BOOLEAN DEFAULT TRUE

### Taula `users`
- `id`: INTEGER PRIMARY KEY
- `username`: TEXT NOT NULL
- `proxy_key`: TEXT UNIQUE NOT NULL (La clau que usarà l'usuari final)
- `enable`: BOOLEAN DEFAULT TRUE

### Taula `usage_logs`
- `id`: INTEGER PRIMARY KEY
- `user_id`: INTEGER
- `model_alias`: TEXT
- `prompt_tokens`: INTEGER
- `completion_tokens`: INTEGER
- `latency_ms`: INTEGER
- `cost`: FLOAT (Opcional)
- `timestamp`: DATETIME DEFAULT CURRENT_TIMESTAMP

## 4. Lògica del Proxy i Adaptadors

### A. Capa d'Adaptació (Provider Traits)
El programa ha d'implementar un `Trait` anomenat `LLMProvider` que defineixi:
1. `prepare_request()`: Tradueix el JSON estàndard d'OpenAI al format del proveïdor.
2. `parse_streaming_chunk()`: Extrau el text i el recompte de tokens de cada fragment del flux de dades.

### B. Gestió de Streaming (SSE)
- El proxy ha de convertir qualsevol resposta (sigui d'Anthropic o Gemini) en un format de sortida **Server-Sent Events (SSE) compatible amb OpenAI**.
- Això permet que eines com Chatbox, LibreChat o scripts de Python funcionin sense canvis.

### C. Captura de Mètriques
- En finalitzar un stream, el proxy ha d'interceptar l'objecte de `usage`.
- Si el proveïdor no envia tokens (com en el cas d'Anthropic en alguns modes), l'agent ha d'intentar comptar els tokens utilitzant una llibreria com `tiktoken-rs` o registrar el temps total de resposta.

## 5. Endpoints d'Administració (Protegits per `ADMIN_KEY`)
- `GET /admin/backends`: Llista els models definits.
- `GET /admin/backend/:id`: Mostre al info d'aquest backend
- `POST /admin/backend`: Registrar un nou model (Local, Anthropic o Gemini).
- `PUT /admin/backend/:id`: Modificia un model ja existent.
- `DELETE /admin/backend/:id`: Esborra un model
- `GET /admin/users`: Llista de tots el usuaris.
- `GET /admin/user/:id`: Mostre al info d'aquest usuari
- `POST /admin/users`: Crear usuaris i claus de proxy.
- `PUT /admin/user/:id`: Modificar usuaris (habilitar/deshabilitar).
- `DELETE /admin/user/:id`: Esborra un usuari.
- `GET /admin/stats`: Dashboard de dades (Tokens/segon per model, consum per usuari).

## 6. Instruccions de Generació per a l'Agent
1. Configura `Cargo.toml` amb `axum`, `tokio`, `sqlx`, `reqwest`, `serde`, `futures-util`.
2. Implementa la lògica de forwarding asíncrona utilitzant el patró **Strategy** per als diferents proveïdors.
3. Assegura't que el binari es pugui executar darrere d'un **túnel de Cloudflare**, gestionant correctament els timeouts i els headers de IP real.

## 7. Primera arrancada
- El sistema pot rebre per parametres la clau del administrador (o la variable d'entorn `ADMIN_KEY`) i opcionalment un port d'escolta (per defecte `8080`), un host (per defecte `0.0.0.0`) i la url de la base de dades (per defecte `sqlite://./llamp.db`).
- Si no existeix la base de dades, el sistema la crearà automàticament amb les taules necessàries.