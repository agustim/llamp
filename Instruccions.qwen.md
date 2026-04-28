# Especificacions Tècniques: Llamp — Rust Universal LLM Gateway

## 1. Objectiu
Crear un proxy d'alt rendiment en **Rust** que centralitzi les crides a models de llenguatge des de serveis externs (internet). El sistema redirigeix peticions a múltiples proveïdors (OpenAI, Anthropic, Google Gemini, Groq, Azure OpenAI, etc.), gestionant la traducció de protocols, l'autenticació d'usuaris, límits de consum i registre detallat.

**Philosophy:** El proxy accepta sempre un format d'entrada **OpenAI-compatible** i normalitza totes les sortides al mateix format. L'usuari final no necessita saber quin proveïdor subjacent s'està utilitzant.

---

## 2. Stack Tecnològic
- **Llenguatge:** Rust (Latest Stable).
- **Framework Web:** `Axum` (asíncron).
- **Client HTTP:** `reqwest` amb suport `stream` + `gzip`.
- **Base de dades:** `SQLite` amb `SQLx` (mode sync, sense runtime overhead).
- **Serialització:** `Serde` + `serde_json`.
- **Configuració:** `config` crate (fitxer `.toml` amb override per variables d'entorn).
- **Logging:** `tracing` + `tracing-subscriber` (JSON format).
- **CLI args:** `clap`.
- **Generació de claus:** `ulid` o `nanoid`.

---

## 3. Providers Suportats (Fase 1)

| Provider | Tipus | Notes |
|---|---|---|
| `openai` | Natiu | Format ja és OpenAI, zero transformació |
| `openai_compatible` | Natiu | Qualsevol API compatible (local LM Studio, Ollama, vLLM, etc.) |
| `anthropic` | Adaptador | Messages API → OpenAI SSE |
| `google_gemini` | Adaptador | Vertex AI / Gemini API → OpenAI SSE |
| `groq` | Natiu | Format OpenAI-compatible, endpoint diferent |
| `azure_openai` | Adaptador | Resource/Deployment path, API key diferent |

> **Fase 1 inclou TOTS aquests proveïdors.** No cal esperar a una fase 2 per a Anthropic o Gemini.

---

## 4. Arquitectura de Base de Dades

### Taula `backends`
- `id`: INTEGER PRIMARY KEY AUTOINCREMENT
- `provider_type`: TEXT NOT NULL (ex: `openai`, `anthropic`, `google_gemini`, `groq`, `azure_openai`, `openai_compatible`)
- `display_name`: TEXT NOT NULL (nom humà per a panells d'admin, ex: "Claude 3.5 Sonnet")
- `model_alias`: TEXT UNIQUE NOT NULL (identificador intern, ex: `claude-3.5-sonnet`)
- `model_name`: TEXT NOT NULL (nom segons el proveïdor, ex: `claude-3-5-sonnet-20241022`)
- `endpoint_url`: TEXT NOT NULL (base URL del servei)
- `api_key`: TEXT (enfobrat a l'app, mai logejat)
- `additional_config`: TEXT (JSON per camps específics del provider, ex: Azure `deployment_id`, Gemini `project_id`)
- `cost_per_input_token`: REAL DEFAULT 0
- `cost_per_output_token`: REAL DEFAULT 0
- `max_request_timeout_s`: INTEGER DEFAULT 300
- `active`: BOOLEAN DEFAULT TRUE
- `created_at`: DATETIME DEFAULT CURRENT_TIMESTAMP
- `updated_at`: DATETIME DEFAULT CURRENT_TIMESTAMP

### Taula `users`
- `id`: INTEGER PRIMARY KEY AUTOINCREMENT
- `username`: TEXT UNIQUE NOT NULL
- `proxy_key`: TEXT UNIQUE NOT NULL (format `llamp-xxxxx`, generat automàticament)
- `enabled`: BOOLEAN DEFAULT TRUE
- `allowed_backends`: TEXT (JSON array d'aliases permesos; `null` = tots)
- `rate_limit_requests_per_minute`: INTEGER DEFAULT 60
- `monthly_token_budget`: INTEGER DEFAULT -1 (`-1` = sense límit)
- `created_at`: DATETIME DEFAULT CURRENT_TIMESTAMP
- `updated_at`: DATETIME DEFAULT CURRENT_TIMESTAMP

### Taula `usage_logs`
- `id`: INTEGER PRIMARY KEY AUTOINCREMENT
- `user_id`: INTEGER REFERENCES users(id)
- `model_alias`: TEXT
- `prompt_tokens`: INTEGER DEFAULT 0
- `completion_tokens`: INTEGER DEFAULT 0
- `total_tokens`: INTEGER DEFAULT 0
- `latency_ms`: INTEGER
- `cost`: REAL DEFAULT 0
- `status`: TEXT DEFAULT 'success' (vals: `success`, `error`, `rate_limited`, `timeout`)
- `error_message`: TEXT
- `timestamp`: DATETIME DEFAULT CURRENT_TIMESTAMP

### Taula `system_config` (per a configuració persistent)
- `key`: TEXT PRIMARY KEY
- `value`: TEXT

---

## 5. Configuració de l'Aplicació

### Fitxer `llamp.toml` (opcional, amb defaults)
```toml
[server]
host = "0.0.0.0"
port = 8080
read_timeout_secs = 300
write_timeout_secs = 300

[admin]
# ADMIN_KEY es pot passar per CLI, env var, o aquest fitxer

[database]
url = "sqlite://./llamp.db"

[logging]
level = "info"          # trace, debug, info, warn, error
format = "json"         # json, pretty
```

### Variables d'entorn (override)
| Variable | Descripció |
|---|---|
| `ADMIN_KEY` | Clau per accedir a endpoints d'administració |
| `LLAMP_PORT` | Port d'escolta (default `8080`) |
| `LLAMP_HOST` | Host (default `0.0.0.0`) |
| `LLAMP_DATABASE_URL` | Connexió BD |
| `LLAMP_LOG_LEVEL` | Nivell de logging |
| `LLAMP_CONFIG` | Ruta al fitxer `llamp.toml` |

### Paràmetres CLI
```
llamp [OPCTIONS]
    --admin-key <KEY>      Clau d'administrador
    --port <PORT>          Port d'escolta (default: 8080)
    --host <HOST>          Host (default: 0.0.0.0)
    --config <PATH>        Fitxer de configuració
    --database <URL>       URL de la base de dades
```

**Prioritat:** CLI > Environment > config file > defaults

---

## 6. Autenticació d'Usuaris

Els clients autentifiquen les seves peticions amb el header:
```
Authorization: Bearer llamp-xxxxx-xxxxx-xxxxx
```

El proxy valida:
1. La `proxy_key` existeix i l'usuari està `enabled`
2. L'usuari no ha exhaurit el `monthly_token_budget` (si configurat)
3. El backend sol·licitat està a la llista `allowed_backends` (si configurada)

---

## 7. Ruteig de Models

L'usuari final fa la petició al path estàndard OpenAI:
```
POST /v1/chat/completions
Content-Type: application/json
Authorization: Bearer llamp-xxxxx

{
  "model": "claude-3.5-sonnet",   // ← aquest és el model_alias
  "messages": [...]
}
```

El proxy:
1. Cerca `model_alias` a la taula `backends` (només filtres `active = true`)
2. Carrega el provider corresponent
3. Transforma la petició segons el provider
4. Foward a l'endpoint del provider
5. Normalitza la resposta a format OpenAI SSE

### Ruteig per backend (avançat)
Permetre que l'admin afegeixi un camp `priority` als backends per suportar:
- **Fallback:** si el backend primari falla, provar el següent (mateix `model_alias` lògic, diferent provider)
- **Load balancing round-robin:** per múltiples instàncies del mateix model

---

## 8. Capa d'Adaptació (Provider Traits)

```rust
trait LLMProvider: Send + Sync {
    // Transforma una petició OpenAI al format del provider
    async fn prepare_request(&self, req: &ChatCompletionRequest, backend: &Backend) -> Result<reqwest::Request>;

    // Parseja un chunk d'SSE del provider i retorna format OpenAI
    fn parse_streaming_chunk(&self, raw: &[u8]) -> Result<Option<OpenAIStreamChunk>>;

    // Extrai usage d'una resposta no-streaming
    fn parse_usage(&self, body: &Value) -> Option<Usage>;

    // Retorna el content type que espera el provider
    fn content_type(&self) -> &str;
}
```

### Format de sortida unificat (OpenAI SSE)
```
data: {"id":"...","object":"chat.completion.chunk","model":"...","choices":[{"index":0,"delta":{"content":"text"}}]}

data: {"id":"...","object":"chat.completion.chunk","model":"...","choices":[{"index":0,"delta":{"content":""},"finish_reason":"stop"}]}

data: [DONE]
```

---

## 9. Captura de Mètriques

- **Tokens:** s'extrauen del camp `usage` de la resposta del provider. Si no disponible, es calculen amb `tiktoken-rs` (per models OpenAI) o `anthropic_tokens` (per Claude).
- **Latència:** temps des de la recepció de la petició fins al primer byte de resposta (`TTFT` — Time To First Token) + temps total.
- **Cost:** calculat automàticament a partir de `cost_per_input_token` i `cost_per_output_token` del backend.
- **Error tracking:** cada fallada es registra a `usage_logs` amb `status` i `error_message`.

---

## 10. Rate Limiting

- **Per usuari:** `rate_limit_requests_per_minute` a la taula `users`
- **Global:** configurable a `llamp.toml`
- Implementat amb `governor` crate (token bucket, in-memory, backed by SQLite per persistència)
- Resposta `429 Too Many Requests` amb header `Retry-After`

---

## 11. Resiliència

- **Retries:** 2 intents automàtics per errors transitoris (5xx, timeout, rate limit del provider)
- **Timeout per backend:** configurable a nivell de backend (`max_request_timeout_s`)
- **Circuit breaker:** si un backend falla >5 vegades en 60s, el proxy el marca com `unavailable` durant 5 minuts (sense tocar `active` a la BD)
- **Health check:** `GET /health` retorna l'estat dels backends actius

---

## 12. Endpoints

### API Pública (requereix `Authorization: Bearer llamp-xxxxx`)
| Mètode | Path | Descripció |
|---|---|---|
| `POST` | `/v1/chat/completions` | Completions de xat (streaming + no-streaming) |
| `GET` | `/v1/models` | Llista de models disponibles per a aquest usuari |

### Endpoints d'Administració (requereix `X-Admin-Key` o `Authorization: Bearer <ADMIN_KEY>`)
| Mètode | Path | Descripció |
|---|---|---|
| `GET` | `/admin/backends` | Llista backends (amb filtre `?active=true`) |
| `GET` | `/admin/backends/:id` | Detall d'un backend (sense `api_key`) |
| `POST` | `/admin/backends` | Afegir backend |
| `PUT` | `/admin/backends/:id` | Actualitzar backend |
| `DELETE` | `/admin/backends/:id` | Eliminar backend |
| `POST` | `/admin/backends/:id/test` | Provar connexió amb el backend |
| `GET` | `/admin/users` | Llista usuaris |
| `GET` | `/admin/users/:id` | Detall usuari (sense `proxy_key` completa) |
| `POST` | `/admin/users` | Crear usuari (genera `proxy_key` automàtic) |
| `PUT` | `/admin/users/:id` | Actualitzar usuari |
| `DELETE` | `/admin/users/:id` | Eliminar usuari |
| `POST` | `/admin/users/:id/regenerate-key` | Regenerar la clau d'un usuari |
| `GET` | `/admin/stats/overview` | Tokens/hora, requicions/minut, cost total |
| `GET` | `/admin/stats/by-user` | Consum agrupat per usuari (amb `?period=today\|week\|month`) |
| `GET` | `/admin/stats/by-model` | Consum agrupat per model |
| `GET` | `/admin/logs` | Logs de peticions recents (amb `?limit=50&user_id=&status=`) |

### Endpoints de Sistema
| Mètode | Path | Descripció |
|---|---|---|
| `GET` | `/health` | Estat del sistema i backends |

---

## 13. Primeres Arrancada

1. Si no existeix la base de dades, es crea automàticament amb les taules necessàries (`SQLX_OFFLINE=false` per development).
2. Si `ADMIN_KEY` no està configurat per cap mitjà (CLI, env, config), el programa **no arranca** i mostra un error.
3. A `POST /admin/users` la primera vegada, es crea l'administrador implícit.
4. El log d'arrancada mostra els backends actius carregats i el port d'escolta.

---

## 14. Consideracions de Seguretat

- **API keys** mai es mostren als endpoints `GET` (retornen `sk-****` truncat)
- **Request/response logging** configurable, amb opció de **no logejar cos dels missatges** (només metadades)
- **CORS** configurable (per defecte restrictiu)
- **Headers Cloudflare:** respecte a `CF-Connecting-IP`, `X-Forwarded-For` per a logs d'IP real
- **Database encryption:** l'opció de cifrar la BD SQLite amb una clau externa (`sqlx` + `sqlcipher`) es deixa com a futura millora

---

## 15. Estructura del Projecte

```
llamp/
├── Cargo.toml
├── llamp.toml                  # Configuració per defecte
├── migrations/                 # SQLx migrations
│   ├── 20260101000000_create_backends.sql
│   ├── 20260101000001_create_users.sql
│   ├── 20260101000002_create_usage_logs.sql
│   └── 20260101000003_create_system_config.sql
├── src/
│   ├── main.rs                 # Entry point, CLI parsing
│   ├── config.rs               # Configuració (file + env + CLI)
│   ├── db.rs                   # Connexió BD, migrations
│   ├── models.rs               # Estructures de dades / serde
│   ├── providers/              # LLMProvider implementations
│   │   ├── mod.rs              # Trait definició
│   │   ├── openai.rs
│   │   ├── anthropic.rs
│   │   ├── gemini.rs
│   │   ├── groq.rs
│   │   └── azure.rs
│   ├── proxy/
│   │   ├── mod.rs
│   │   ├── router.rs           # Ruteig model_alias → backend
│   │   ├── streaming.rs        # SSE handling, normalització
│   │   └── metrics.rs          # Captura tokens, latència, cost
│   ├── auth.rs                 # Validació proxy_key
│   ├── rate_limit.rs           # Rate limiting
│   ├── admin/                  # Endpoints d'administració
│   │   ├── mod.rs
│   │   ├── backends.rs
│   │   ├── users.rs
│   │   └── stats.rs
│   └── server.rs               # Axum app setup, middleware
└── tests/
    ├── integration_test.rs
    └── providers/
        ├── openai_test.rs
        └── anthropic_test.rs
```

---

## 16. Ordre d'Implementació

1. **Skeleton:** `Cargo.toml`, `main.rs`, CLI parsing, `config.rs`, estructura de carpetes
2. **Database:** Migrations, `db.rs`, models de dades
3. **Admin API:** CRUD backends + usuaris (sense proxy, només gestió)
4. **Provider OpenAI:** Implementació base (és el format de referència)
5. **Proxy core:** Ruteig, forwarding, streaming SSE normalitzat
6. **Autenticació:** Middleware `proxy_key`, rate limiting
7. **Mètriques:** Captura d'usage, cost, latència → `usage_logs`
8. **Provider Anthropic:** Adaptador Messages API
9. **Provider Gemini:** Adaptador Vertex/AI API
10. **Provider Groq + Azure:** Adaptadors
11. **Resiliència:** Retries, circuit breaker, health checks
12. **Admin Stats:** Endpoints d'estadístiques
13. **Polishing:** Logging, errors, Dockerfile, README

---

## 17. Variables d'Entorn per a Proveïdors

Cada provider pot tenir variables d'entorn com a fallback si no s'ha configurat cap backend:

| Variable | Provider |
|---|---|
| `OPENAI_API_KEY` | OpenAI |
| `ANTHROPIC_API_KEY` | Anthropic |
| `GOOGLE_GENAI_API_KEY` | Google Gemini |
| `GROQ_API_KEY` | Groq |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI |
| `AZURE_OPENAI_ENDPOINT` | Azure OpenAI |

Si existeixen, el programa crea automàticament els backends corresponents a l'arrancada (si no existeixen ja a la BD).
