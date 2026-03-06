---
description: Instrução para compreensão da arquitetura do projeto Hagitori e planejamento de migração para Android. Destinado a fornecer contexto detalhado.
---

# Hagitori — Arquitetura & Roadmap Android

> Documento de referência completo para compreensão da arquitetura do projeto e planejamento
> de migração para Android. Projetado para servir de contexto para LLMs e desenvolvedores.

---

## Sumário

1. [Visão Geral do Projeto](#1-visão-geral-do-projeto)
2. [Stack Tecnológico](#2-stack-tecnológico)
3. [Arquitetura do Backend (Rust)](#3-arquitetura-do-backend-rust)
4. [Arquitetura do Frontend (React)](#4-arquitetura-do-frontend-react)
5. [Fluxos de Dados End-to-End](#5-fluxos-de-dados-end-to-end)
6. [Sistema de Extensões JavaScript](#6-sistema-de-extensões-javascript)
7. [Sistema de Browser e Cloudflare Bypass](#7-sistema-de-browser-e-cloudflare-bypass)
8. [Persistência e Storage](#8-persistência-e-storage)
9. [Segurança e Sandboxing](#9-segurança-e-sandboxing)
10. [Roadmap Android — Fases de Implementação](#10-roadmap-android--fases-de-implementação)
11. [Adaptações Específicas para Android](#11-adaptações-específicas-para-android)
12. [Features Futuras (Reader, Search, Local Files)](#12-features-futuras-reader-search-local-files)
13. [Referência de Comandos Tauri](#13-referência-de-comandos-tauri)
14. [Referência de Crates Rust](#14-referência-de-crates-rust)
15. [Referência do Frontend](#15-referência-do-frontend)
16. [Constantes e Configurações Hardcoded](#16-constantes-e-configurações-hardcoded)

---

## 1. Visão Geral do Projeto

**Hagitori** é um aplicativo desktop para download e gerenciamento de mangás, construído com
Tauri 2 (Rust backend + React frontend). Suas características principais são:

- **Sistema de extensões JavaScript** — extensões de terceiros escritas em JS/TS que implementam
  parsers para diferentes sites de mangá (similares ao Tachiyomi/Mihon)
- **Engine de download** — download concorrente de capítulos com suporte a dois paths:
  HTTP direto ou via automação de browser
- **Cloudflare bypass** — automação de Chrome headful para resolver challenges anti-bot
  e extrair cookies de sessão (`cf_clearance`)
- **TLS fingerprinting** — emulação de TLS fingerprint do Chrome 145 via `wreq`
  para contornar detecção básica de bots
- **Biblioteca local** — gerenciamento de mangás com persistência SQLite
- **Catálogo de extensões** — sistema de descoberta, instalação e atualização de extensões
  a partir de repositórios remotos (similar ao app store de extensões do Mihon)
- **Empacotamento CBZ** — exportação de capítulos baixados no formato Comic Book ZIP
  com metadata ComicInfo.xml

### Identificadores

- **Package name**: `com.hagitori.app`
- **Versão atual**: `0.0.2-beta`
- **Repositório**: `https://github.com/hagitori/hagitori`
- **Android mínimo**: Android 10 (API 29)

---

## 2. Stack Tecnológico

### Backend (Rust)

| Dependência | Versão | Função | Android Compatível |
|------------|--------|--------|-------------------|
| `tauri` | 2.10.3 | Framework desktop/mobile | ✅ (com suporte mobile) |
| `rquickjs` | 0.11.0 | Engine JavaScript QuickJS | ✅ (compila para ARM) |
| `wreq` | 6.0.0-rc.28 | HTTP client com TLS emulation | ✅ |
| `chromiumoxide` | 0.9 | Automação Chrome/Chromium | ❌ (requer Chrome instalado) |
| `rusqlite` | 0.38.0 (bundled) | SQLite embarcado | ✅ |
| `tokio` | 1.50.0 | Runtime assíncrono | ✅ |
| `scraper` | 0.25.0 | HTML parser (CSS selectors) | ✅ |
| `url` | 2.5 | URL parsing | ✅ |
| `serde` / `serde_json` | 1.0 | Serialização | ✅ |
| `zip` | 2.6 | Criação de arquivos CBZ | ✅ |
| `image` | 0.25.9 | Conversão de imagens | ✅ |
| `sha2` / `md-5` / `hmac` | latest | Hashing criptográfico | ✅ |
| `chrono` | latest | Data/hora | ✅ |
| `lru` | latest | Cache LRU | ✅ |
| `tracing` | latest | Logging estruturado | ✅ |
| `base64` | latest | Encoding/decoding | ✅ |

**Features do rquickjs**: `futures`, `parallel`, `classes`, `macro`

### Frontend (React/TypeScript)

| Dependência | Função |
|------------|--------|
| React 18 | UI framework |
| TypeScript | Tipagem |
| Vite | Bundler |
| React Router DOM | Roteamento |
| TanStack Query | Cache de queries |
| Zustand | State management |
| Immer | Imutabilidade |
| Tailwind CSS | Estilização |
| Lucide React | Ícones |
| Framer Motion | Animações |

### Tauri Plugins

| Plugin | Função | Android Compatível |
|--------|--------|-------------------|
| `tauri-plugin-opener` | Abrir URLs/arquivos | ✅ |
| `tauri-plugin-dialog` | Diálogos nativos | ⚠️ (adaptação necessária) |
| `tauri-plugin-process` | Controle de processo | ⚠️ (limitado no mobile) |
| `tauri-plugin-updater` | Auto-update | ❌ (desktop only) |

---

## 3. Arquitetura do Backend (Rust)

### Workspace Structure

O projeto Rust é organizado como um Cargo workspace com **9 crates** independentes
mais o binário Tauri principal:

```
src-tauri/
├── Cargo.toml          ← Workspace root
├── src/                ← Binário Tauri (commands, state)
│   ├── main.rs         ← Entry point (chama lib::run())
│   ├── lib.rs          ← Setup Tauri, AppState, invoke_handler
│   ├── utils.rs        ← Funções utilitárias
│   ├── sync_commands.rs← 9 comandos de sync/catálogo
│   └── commands/
│       ├── mod.rs
│       ├── manga.rs    ← 5 comandos de manga
│       ├── download.rs ← 2 comandos de download
│       ├── config.rs   ← 3 comandos de config
│       └── library.rs  ← 11 comandos de biblioteca
│
└── crates/
    ├── core/           ← Entidades, erros, traits
    ├── http/           ← HTTP client + session store
    ├── browser/        ← Automação Chrome + Cloudflare bypass
    ├── extensions/     ← Runtime QuickJS + sistema de extensões
    ├── providers/      ← Registry de providers (extensões carregadas)
    ├── download/       ← Engine de download de capítulos
    ├── config/         ← Gerenciamento SQLite (5 bancos de dados)
    ├── sync/           ← Catálogo, instalação, atualização de extensões
    └── grouper/        ← Empacotamento CBZ + ComicInfo.xml
```

### AppState (Estado Global Compartilhado)

Definido em `src/lib.rs`, é o estado central do app gerenciado pelo Tauri:

```rust
pub struct AppState {
    pub registry: RwLock<ProviderRegistry>,     // extensões carregadas
    pub http_client: Arc<HttpClient>,            // wreq com TLS emulation
    pub config: Arc<ConfigManager>,              // settings SQLite
    pub ext_registry: Arc<ExtensionRegistry>,    // extensões instaladas SQLite
    pub download_history: Arc<DownloadHistory>,  // histórico SQLite
    pub session_store: Arc<SessionStore>,         // cookies/headers persistidos SQLite
    pub library: Arc<LibraryManager>,            // biblioteca de mangás SQLite
    pub manga_cache: RwLock<LruCache<String, Manga>>,     // cache 256 items
    pub provider_cache: RwLock<LruCache<String, String>>,  // cache 256 items
    pub cancel_token: Mutex<CancellationToken>,  // cancelamento de downloads
    pub browser_manager: Arc<Mutex<Option<Arc<BrowserManager>>>>, // Chrome headful
}
```

### Ciclo de Vida da Aplicação

1. **Startup** (`lib.rs::run()`):
   - Inicializa `tracing_subscriber` com `RUST_LOG`
   - Cria `HttpClient` com emulação Chrome145
   - Determina `data_dir` (platform-specific)
   - Cria diretório de perfil do browser
   - Inicializa `ConfigManager`, `ExtensionRegistry`, `DownloadHistory`, `SessionStore`, `LibraryManager`
   - Carrega extensões de `$DATA_DIR/extensions/`
   - Restaura sessões HTTP persistidas do SQLite para memória
   - Registra todos os 25+ comandos Tauri
   - Constrói e executa a aplicação Tauri

2. **Runtime**:
   - Frontend faz chamadas via `invoke("command_name", payload)`
   - Cada comando acessa `AppState` via `tauri::State<AppState>`
   - Operações assíncronas usam `tokio` runtime

3. **Shutdown** (`RunEvent::Exit`):
   - Exporta sessões HTTP in-memory para SQLite (`session_store.save()`)
   - Log de sessões persistidas

### Diagrama de Dependências entre Crates

```
                    ┌──────────────┐
                    │  hagitori    │ (binário Tauri)
                    │  (src/)      │
                    └──────┬───────┘
                           │ depends on
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼────┐      ┌─────▼─────┐     ┌──────▼──────┐
   │providers│      │  download │     │    sync     │
   │         │      │           │     │             │
   └────┬────┘      └─────┬─────┘     └──────┬──────┘
        │                 │                   │
   ┌────▼────┐      ┌─────▼─────┐     ┌──────▼──────┐
   │extensions│     │  grouper  │     │   config    │
   │         │      │           │     │             │
   └─┬──┬───┘      └───────────┘     └─────────────┘
     │  │
     │  └──────────┐
     │             │
┌────▼────┐  ┌─────▼─────┐
│  http   │  │  browser  │
│         │  │           │
└────┬────┘  └───────────┘
     │
┌────▼────┐
│  core   │
│         │
└─────────┘
```

---

## 4. Arquitetura do Frontend (React)

### Roteamento (App.tsx)

```
/                → Home (biblioteca + cards de mangá)
/search          → Search (busca por URL)
/library/:source → ExtensionManga (mangás de uma extensão específica)
/manga/:id       → MangaDetail (detalhes + lista de capítulos)
/downloads       → Downloads (fila + progresso)
/extensions      → Extensions (catálogo + instaladas)
/settings        → Settings (configurações)
```

Todas as rotas são lazy-loaded exceto Home. O layout usa `<Outlet />` do React Router
com `<Layout />` como wrapper.

### Layout Atual (Desktop)

```
┌─────────────────────────────────────────────┐
│  ┌─────────┐  ┌─────────────────────────┐  │
│  │         │  │                         │  │
│  │ Sidebar │  │    Main Content         │  │
│  │         │  │    (<Outlet />)         │  │
│  │ - Home  │  │                         │  │
│  │ - Search│  │                         │  │
│  │ - Downl │  │                         │  │
│  │ - Exts  │  │                         │  │
│  │ - Setts │  │                         │  │
│  │         │  │                         │  │
│  └─────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────┘
```

### State Management (Zustand)

**4 stores**:

1. **`download-store.ts`** — Fila de downloads, controle de retry
   - `downloadQueue: DownloadItem[]`
   - `addToQueue()`, `removeFromQueue()`, `startDownload()`, `retryFailed()`
   - Progresso atualizado via Tauri events (`download-progress`)

2. **`library-store.ts`** — Biblioteca de mangás (com immer)
   - `entries: Map<string, LibraryEntry>`
   - `sourceNames: Map<string, string>`
   - `extensionLangs: Map<string, string[]>`
   - CRUD operations + `migrateFromLocalStorage()`
   - Persistência via comandos Tauri (SQLite)

3. **`settings-store.ts`** — Configurações (persist localStorage)
   - `downloadPath`, `imageFormat`, `language`, `autoUpdateExtensions`, `catalogUrl`
   - Sincronização bidirecional com backend via `useConfig` hook

4. **`toast-store.ts`** — Notificações toast
   - `toasts: Toast[]`
   - `addToast()`, `removeToast()`
   - Auto-dismiss com timeout

### Custom Hooks

| Hook | Função |
|------|--------|
| `useConfig()` | Sincroniza settings frontend ↔ backend SQLite |
| `useDownloadProgress()` | Listener de `Tauri.listen("download-progress")` |
| `useExtensions()` | `invoke("list_extensions")` → lista de extensões ativas |
| `useExtensionFilters()` | Filtros de busca/idioma no catálogo |
| `useSync()` | `invoke("fetch_catalog")`, install/update/remove |
| `useAppUpdater()` | Verifica atualizações do app via `tauri-plugin-updater` |
| `useTranslation()` | i18n com suporte a `en` e `pt-br` |

### IPC Bindings (lib/tauri.ts)

Todas as chamadas ao backend são centralizadas em `src/lib/tauri.ts`:

```typescript
import { invoke } from "@tauri-apps/api/core";

export async function getManga(url: string): Promise<Manga> { ... }
export async function getChapters(mangaId: string): Promise<Chapter[]> { ... }
export async function downloadChapters(items: DownloadRequest[]): Promise<void> { ... }
export async function fetchCatalog(): Promise<CatalogEntry[]> { ... }
// ... etc (todas as 25+ funções)
```

### Tipos TypeScript (types/index.ts)

```typescript
interface Manga {
  id: string;
  name: string;
  cover: string | null;
  source: string;
  url?: string;
}

interface Chapter {
  id: string;
  number: string;
  name: string;
  title?: string;
  date?: string;
  scanlator?: string;
}

interface Pages {
  chapterId: string;
  chapterNumber: string;
  mangaName: string;
  pages: string[];
  headers?: Record<string, string>;
  useBrowser: boolean;
  scanlator?: string;
}

interface MangaDetails {
  id: string;
  name: string;
  synopsis?: string;
  author?: string;
  artist?: string;
  tags: string[];
  altTitles: string[];
  status?: string;
  source: string;
}

interface ExtensionMeta {
  id: string;
  name: string;
  lang: string;
  version: string;
  versionId: number;
  domains: string[];
  languages: string[];
  features: string[];
  icon?: string;
}

interface DownloadProgress {
  mangaName: string;
  chapterNumber: string;
  currentPage: number;
  totalPages: number;
  status: DownloadStatus;
  savePath?: string;
}

type DownloadStatus =
  | "Queued"
  | "Downloading"
  | "Processing"
  | { Failed: string }
  | "Completed";
```

---

## 5. Fluxos de Dados End-to-End

### Fluxo: Buscar Mangá por URL

```
[Frontend] Input URL → invoke("get_manga", { url })
    ↓
[Tauri Command] commands::manga::get_manga()
    ├─ Check manga_cache (LRU) → hit? return cached
    ├─ Find provider via ProviderRegistry.find_by_url(url)
    │   └─ Matches URL against registered extension domains
    ├─ provider.get_manga(url).await
    │   ↓
    │ [JsExtension]
    │   ├─ acquire_worker() → pool or spawn new JsWorker
    │   ├─ worker.call("getManga", [url]).await
    │   │   ↓
    │   │ [JsWorker] (QuickJS AsyncContext)
    │   │   ├─ Reset deadline (30s timeout)
    │   │   ├─ Call __extension__.getManga(url)
    │   │   ├─ Resolve Promise if returned
    │   │   ├─ js_value_to_json() → serde_json::Value
    │   │   └─ Return result
    │   ├─ json_to_manga(result) → Manga struct
    │   ├─ Reset consecutive_failures to 0
    │   └─ release_worker() → return to pool
    ├─ Cache manga in manga_cache
    └─ Return Manga to frontend
```

### Fluxo: Download de Capítulo

```
[Frontend] Select chapters → downloadStore.addToQueue()
    → invoke("download_chapters", { items })
    ↓
[Tauri Command] commands::download::download_chapters()
    For each chapter:
    ├─ provider.get_pages(chapter).await
    │   └─ JsExtension calls getManga → getPages in QuickJS
    ├─ Determine download path: config.download_dir / manga_name / chapter_number
    ├─ Check pages.use_browser
    │
    ├─ [HTTP Path] (use_browser = false)
    │   ├─ Semaphore limit (3-5 concurrent)
    │   ├─ For each page URL:
    │   │   ├─ http_client.get(url, opts) with custom headers from pages.headers
    │   │   ├─ Handle 429 rate limiting (Retry-After header)
    │   │   ├─ Convert image format if configured (PNG/JPEG/WebP)
    │   │   ├─ Save to disk: chapter_dir/001.png
    │   │   └─ emit("download-progress", { currentPage, totalPages, status })
    │   └─ Max 3 retries with exponential backoff
    │
    ├─ [Browser Path] (use_browser = true)
    │   ├─ Get or create BrowserManager
    │   ├─ Create page pool (3-5 pages)
    │   ├─ For each page URL:
    │   │   ├─ Navigate Chrome page to URL
    │   │   ├─ Wait for network response via CDP (ResponseReceived event)
    │   │   ├─ Extract response body (base64 decoded)
    │   │   ├─ Save to disk
    │   │   └─ emit("download-progress")
    │   └─ Close pages (browser persists for cookie reuse)
    │
    ├─ After all pages downloaded:
    │   ├─ Check output format preference
    │   ├─ [CBZ] grouper::create_archive()
    │   │   ├─ Create ZIP with chapter images
    │   │   ├─ Generate ComicInfo.xml metadata
    │   │   ├─ Write to temp file → atomic rename
    │   │   └─ Delete original image folder
    │   └─ [Folder] Keep as-is
    │
    └─ Record in download_history SQLite
```

### Fluxo: Instalação de Extensão do Catálogo

```
[Frontend] Click "Install" on extension card
    → invoke("install_catalog_extension", { entry })
    ↓
[Tauri Command] sync_commands::install_catalog_extension()
    ├─ ExtensionInstaller::install(entry, http_client, catalog_url)
    │   ├─ Create temp directory
    │   ├─ For each file in entry.files:
    │   │   ├─ Download file from catalog_url/entry.path/file.name
    │   │   ├─ Validate file size (< max allowed)
    │   │   ├─ Compute SHA-256 hash
    │   │   ├─ Compare hash with entry.checksums[file.name]
    │   │   └─ Save to temp dir
    │   ├─ Validate total extension size
    │   ├─ Parse manifest from downloaded package.json
    │   ├─ Verify manifest.id matches entry.id
    │   ├─ Atomic move: temp_dir → $DATA_DIR/extensions/{lang}/{ext_name}/
    │   └─ On any failure: delete temp dir (rollback)
    ├─ ext_registry.insert(entry_metadata) → SQLite
    ├─ Reload ProviderRegistry (re-scan extensions directory)
    └─ Return success
```

### Fluxo: Cloudflare Bypass

```
[Extension fetch()] → 403 or CF challenge detected
    ↓
[Browser Manager]
    ├─ Detect Chrome installation:
    │   ├─ Windows: Registry HKLM\Software\Google\Chrome + PowerShell fallback
    │   ├─ Linux: which google-chrome / chromium
    │   └─ macOS: /Applications/Google Chrome.app
    ├─ Launch Chrome with stealth config:
    │   ├─ --disable-blink-features=AutomationControlled
    │   ├─ --user-data-dir=$DATA_DIR/browser_profile
    │   ├─ Window size 1280×720
    │   ├─ User-Agent matching real Chrome version
    │   └─ Headful mode (headless is detected by CF)
    ├─ Navigate to target URL
    ├─ Cloudflare detection loop (90s timeout):
    │   ├─ Check page title for "Just a moment" / "Checking your browser"
    │   ├─ Find Turnstile iframe in DOM via CDP
    │   ├─ If found:
    │   │   ├─ Get iframe bounding rect
    │   │   ├─ Calculate click coordinates (center of checkbox)
    │   │   ├─ Simulate human-like mouse movement:
    │   │   │   └─ mouseMoved → pause(100ms) → mousePressed → pause(50ms) → mouseReleased
    │   │   ├─ Wait for title change (poll 500ms for 5s)
    │   │   └─ Repeat if still challenged
    │   └─ Loop until page title changes (success) or timeout
    ├─ Extract cookies via CDP Network.getCookies()
    │   └─ Key cookie: "cf_clearance"
    ├─ Import cookies into HttpClient.session_store
    └─ Browser stays open for session cookie reuse
```

---

## 6. Sistema de Extensões JavaScript

### Arquitetura do Runtime QuickJS

```
JsExtension (Rust)
├── meta: ExtensionMeta (id, name, version, domains, features)
├── script: Arc<String> (JavaScript source + preamble)
├── runtime: Arc<JsRuntime> → RuntimeData
│   ├── http_client: Arc<HttpClient>
│   └── browser_manager: Arc<Mutex<Option<BrowserManager>>>
├── workers: Mutex<Vec<JsWorker>> (pool, max 5)
├── worker_semaphore: Semaphore(5)
└── consecutive_failures: AtomicU32 (circuit breaker, max 5)
```

### JsWorker Lifecycle

Cada worker é uma tokio task com seu próprio `AsyncRuntime` + `AsyncContext`:

```rust
JsWorker::spawn()
    ├─ Create AsyncRuntime
    ├─ Set interrupt handler (30s call timeout, 60s init timeout)
    ├─ Set promise rejection tracker (logs unhandled rejections)
    ├─ Set memory limit (64 MB)
    ├─ Set max stack size (2 MB)
    ├─ Create AsyncContext::full()
    ├─ Register all native APIs:
    │   ├─ utils: console, atob, btoa, setTimeout, clearTimeout, sleep,
    │   │         setInterval, clearInterval, URLSearchParams,
    │   │         AbortController, TextEncoder, TextDecoder, URL
    │   ├─ entities: Manga, Chapter, Pages (native classes)
    │   ├─ http: fetch() (domain-whitelisted)
    │   ├─ html: parseHtml() → Document (native class with select/selectOne)
    │   ├─ cookies: getCookies(), setCookies()
    │   ├─ session: getSession(), setSession()
    │   ├─ date: Date polyfill
    │   ├─ browser: openBrowser(), interceptRequest() [conditional]
    │   └─ crypto: sha256(), md5(), hmac() [conditional]
    ├─ Evaluate extension script (creates __extension__ global)
    └─ Enter command loop:
        while recv(WorkerCmd::Call { method, args }) {
            ├─ Reset deadline to now + 30s
            ├─ Call __extension__.{method}(args...)
            ├─ Resolve Promise if needed (MaybePromise)
            ├─ Convert result to JSON (js_value_to_json)
            ├─ Drive pending jobs (rt.idle())
            └─ Send result back via oneshot channel
        }
```

### API Surface Exposta ao JavaScript

Cada extensão tem acesso às seguintes APIs globais:

```javascript
// ── HTTP ──
fetch(url, options?) → Promise<FetchResponse>
  // options: { method, headers, body, form, referer }
  // FetchResponse: { status, headers, text(), json(), bytes() }
  // Domain validation: só domínios declarados no manifest

// ── HTML Parsing ──
parseHtml(htmlString) → Document
  // Document.select(css) → Element[]
  // Document.selectOne(css) → Element | null
  // Document.text() → string
  // Document.html() → string
  // Element: { text, html, attr(name), children }

// ── Entidades ──
new Manga({ id, name, cover? })
new Chapter({ id, number, name, title?, date?, scanlator? })
new Pages({ id, number, name, urls, headers?, useBrowser? })

// ── Timers ──
setTimeout(fn, ms?) → Promise<id>
clearTimeout(id) → void
setInterval(fn, ms?) → Promise<id>  // COMPAT-03
clearInterval(id) → Promise<void>   // COMPAT-03
sleep(ms) → Promise<void>

// ── Encoding ──
atob(base64) → string
btoa(string) → base64
new TextEncoder().encode(string) → number[]   // COMPAT-06
new TextDecoder().decode(array) → string       // COMPAT-06

// ── URL ──
new URL(input, base?) → URLObject              // COMPAT-07
  // { href, protocol, hostname, port, host, pathname, search, hash, origin, searchParams }
new URLSearchParams(init?) → USPObject
  // { get, set, append, delete, has, toString, getAll, keys, values, entries }

// ── Misc ──
new AbortController() → { signal: { aborted, reason }, abort(reason?) }  // COMPAT-05
console.log(...args), console.warn(...args), console.error(...args)

// ── Variáveis Globais ──
__lang__  // idioma ativo da extensão
__id__    // ID da extensão

// ── APIs Condicionais ──
// requires_browser:
openBrowser(url) → Promise<BrowserSession>
interceptRequest(url, patterns) → Promise<InterceptResult>

// requires_crypto:
sha256(input) → string
md5(input) → string
hmac(key, data, algorithm) → string
```

### Estrutura de uma Extensão

```
extensions/{lang}/{ext_name}/
├── package.json    ← Manifest (nome, versão, domínios, features)
├── index.js        ← Script principal
└── icon.png        ← Ícone (opcional)
```

**package.json** (ExtensionManifest):
```json
{
  "name": "MangaSite",
  "id": "mangasite",
  "version": "1.0.0",
  "versionId": 1,
  "lang": "en",
  "languages": ["en", "pt-br"],
  "hagitori": {
    "domains": ["mangasite.com", "api.mangasite.com"],
    "features": ["browser", "crypto"]
  }
}
```

**index.js** (Extension Script):
```javascript
class Extension {
  constructor() {
    this.baseUrl = "https://mangasite.com";
  }

  async getManga(url) {
    const resp = await fetch(url);
    const doc = parseHtml(await resp.text());
    const title = doc.selectOne("h1.title").text;
    const cover = doc.selectOne("img.cover").attr("src");
    return new Manga({ id: url, name: title, cover });
  }

  async getChapters(mangaId) {
    const resp = await fetch(mangaId + "/chapters");
    const data = await resp.json();
    return data.map(ch => new Chapter({
      id: ch.url,
      number: ch.num.toString(),
      name: ch.title
    }));
  }

  async getPages(chapter) {
    const resp = await fetch(chapter.id);
    const doc = parseHtml(await resp.text());
    const imgs = doc.select("img.page").map(el => el.attr("src"));
    return new Pages({
      id: chapter.id,
      number: chapter.number,
      name: chapter.name,
      urls: imgs
    });
  }
}

var __extension__ = new Extension();
```

---

## 7. Sistema de Browser e Cloudflare Bypass

### Componentes do Browser Crate

| Arquivo | Função |
|---------|--------|
| `chrome.rs` | Detecção da instalação do Chrome (paths do sistema) |
| `stealth.rs` | Configuração anti-detecção (UA, headers, flags) |
| `cloudflare.rs` | Solver de challenges Cloudflare (Turnstile interaction) |
| `intercept.rs` | Interceptação de requests/responses via CDP |
| `manager.rs` | Gerenciamento de browser instances e page pool |

### Detecção do Chrome

**Windows**:
1. Registry: `HKLM\SOFTWARE\Google\Chrome\BLBeacon` → `version` key
2. Fallback: PowerShell `Get-ItemProperty` em paths conhecidos
3. Paths: `C:\Program Files\Google\Chrome\Application\chrome.exe`,
   `C:\Program Files (x86)\...`, `%LOCALAPPDATA%\...`

**Linux**:
1. `which google-chrome`
2. `which google-chrome-stable`
3. `which chromium`
4. `which chromium-browser`

### Configuração Anti-Detecção (Stealth)

```rust
LaunchOptions {
    headless: false,  // headless é detectado pelo Cloudflare
    args: [
        "--disable-blink-features=AutomationControlled",
        "--no-first-run",
        "--disable-default-apps",
        "--disable-extensions",
        "--disable-component-extensions-with-background-pages",
        "--disable-background-networking",
        format!("--window-size={},{}", width, height),
        format!("--user-data-dir={}", profile_dir),
    ],
    user_agent: Some(real_chrome_ua), // matched to detected Chrome version
}
```

### Cloudflare Challenge Solver

Sequência do solver (`cloudflare.rs`):

1. **Navegação**: Abre URL alvo no Chrome
2. **Detecção**: Verifica se título contém "Just a moment" ou "Checking your browser"
3. **Turnstile**: Busca iframe com `#turnstile-wrapper` ou `iframe[src*="challenges.cloudflare"]`
4. **Click Simulation**:
   - JavaScript: `getBoundingClientRect()` do iframe
   - CDP: `Input.dispatchMouseEvent` com sequência humanizada
   - `mouseMoved` (200ms delay) → `mousePressed` (50ms delay) → `mouseReleased`
5. **Polling**: Verifica título a cada 500ms por 5s
6. **Cookie Extraction**: `Network.getCookies()` → filtra `cf_clearance`
7. **Session Import**: Cookies importados no `HttpClient.session_store()`
8. **Timeout**: 90 segundos máx, depois retorna erro

### Interceptação de Requests

O `intercept.rs` registra listeners CDP para:
- `Network.requestWillBeSent` → captura URL, headers, body
- `Network.responseReceived` → captura status, headers
- `Network.getResponseBody` → captura response body (base64)

Usado pelo download engine quando `pages.use_browser = true`.

### Impacto no Android

O browser crate inteiro depende de `chromiumoxide` que requer Chrome instalado como
executável no sistema. No Android, Chrome existe como APK mas não pode ser controlado
via CDP. Este é o **único crate que precisa de reimplementação** para Android.

---

## 8. Persistência e Storage

### Bancos de Dados SQLite (5 databases)

Todos gerenciados pelo crate `config`:

```
$DATA_DIR/
├── config.db       ← Configurações key/value
├── extensions.db   ← Extensões instaladas do catálogo
├── library.db      ← Biblioteca (mangás + capítulos + detalhes)
├── sessions.db     ← Sessões HTTP (cookies, headers, UA por domínio)
└── history.db      ← Histórico de downloads
```

#### config.db

```sql
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
-- Keys: "download_dir", "image_format", "catalog_url", "auto_update_extensions"
```

#### extensions.db

```sql
CREATE TABLE IF NOT EXISTS installed_extensions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    version_id INTEGER NOT NULL,
    lang TEXT NOT NULL,
    icon TEXT,
    auto_update INTEGER DEFAULT 1,
    installed_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

#### library.db

```sql
CREATE TABLE IF NOT EXISTS manga (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    cover TEXT,
    source TEXT NOT NULL DEFAULT '',
    url TEXT,
    added_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS chapters (
    id TEXT PRIMARY KEY,
    manga_id TEXT NOT NULL REFERENCES manga(id),
    number TEXT NOT NULL,
    name TEXT NOT NULL,
    title TEXT,
    date TEXT,
    scanlator TEXT
);

CREATE TABLE IF NOT EXISTS details (
    manga_id TEXT PRIMARY KEY REFERENCES manga(id),
    synopsis TEXT,
    author TEXT,
    artist TEXT,
    status TEXT,
    tags TEXT,           -- JSON array
    alt_titles TEXT,     -- JSON array
    source TEXT DEFAULT ''
);

CREATE TABLE IF NOT EXISTS source_meta (
    source TEXT PRIMARY KEY,
    display_name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS extension_langs (
    extension_id TEXT NOT NULL,
    lang TEXT NOT NULL,
    PRIMARY KEY (extension_id, lang)
);
```

#### sessions.db

```sql
CREATE TABLE IF NOT EXISTS sessions (
    domain TEXT PRIMARY KEY,
    cookies TEXT,        -- serialized cookie string
    headers TEXT,        -- JSON object
    user_agent TEXT,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

#### history.db

```sql
CREATE TABLE IF NOT EXISTS downloads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    manga_name TEXT NOT NULL,
    chapter_number TEXT NOT NULL,
    status TEXT NOT NULL,  -- "completed", "failed"
    save_path TEXT,
    downloaded_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### Session Store (In-Memory + SQLite)

O `HttpClient` mantém um `DomainSessionStore` in-memory:

```rust
pub struct DomainSessionStore {
    sessions: RwLock<HashMap<String, DomainSession>>,
}

pub struct DomainSession {
    pub cookies: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub user_agent: Option<String>,
}
```

- **Runtime**: Sessions vivem em memória para performance
- **Startup**: Sessions restauradas do SQLite
- **Shutdown**: Sessions exportadas para SQLite (`Run Event::Exit`)
- **Browser bypass**: Cookies do Cloudflare são importados aqui
- **Requests subsequentes**: `HttpClient.send_request()` aplica cookies/headers da session

### Paths por Plataforma

```
Windows:  %APPDATA%/hagitori/         (C:\Users\<user>\AppData\Roaming\hagitori\)
Linux:    ~/.config/hagitori/
macOS:    ~/Library/Application Support/hagitori/
Android:  /data/data/com.hagitori.app/  (via Tauri path API)
```

---

## 9. Segurança e Sandboxing

### Isolamento de Extensões

| Mecanismo | Detalhe |
|-----------|---------|
| **QuickJS Sandbox** | JS roda em VM isolada, sem acesso ao sistema de arquivos ou rede diretamente |
| **Domain Whitelisting** | `fetch()` só permite requests para domínios declarados no manifest |
| **Memory Limit** | 64 MB por worker (QuickJS `set_memory_limit`) |
| **Stack Limit** | 2 MB por worker (`set_max_stack_size`) |
| **Timeout** | 30s por chamada de método, 60s para inicialização |
| **Circuit Breaker** | Extensão desabilitada após 5 falhas consecutivas |
| **Worker Pool** | Máximo 5 workers simultâneos por extensão |
| **Feature Gates** | `browser` e `crypto` precisam ser declarados no manifest |

### Validação de Extensões (Instalação)

| Check | Detalhe |
|-------|---------|
| **SHA-256 checksum** | Cada arquivo validado contra hash do catálogo |
| **Size limits** | Limites por arquivo e por extensão total |
| **Manifest validation** | ID, versão, domínios devem corresponder ao catálogo |
| **Path traversal** | Nomes de arquivo sanitizados contra `../` |
| **Atomic install** | Download → temp dir → validação → atomic move (rollback em falha) |

### Promise Rejection Tracking

```rust
rt.set_host_promise_rejection_tracker(Some(Box::new(
    |_ctx, _promise, reason, is_handled| {
        if !is_handled {
            tracing::warn!(target: "extension", "Unhandled promise rejection: {reason}");
        }
    },
)));
```

---

## 10. Roadmap Android — Fases de Implementação

### Pré-requisitos

1. **Android SDK 34** + **NDK 27** instalados
2. **Rust targets**: `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`
3. **JDK 17** (para Gradle)
4. **Variáveis de ambiente**: `ANDROID_HOME`, `NDK_HOME`, `JAVA_HOME`
5. **Versão mínima**: Android 10 (API 29) — garante suporte a Scoped Storage,
   WebView moderno (Chrome 80+), e APIs de rede atualizadas

---

### Fase 1: Inicialização Tauri Mobile

**Objetivo**: Compilar e executar o app no Android com todas as features Rust que
já são cross-platform.

**Passos**:

1. Executar `pnpm tauri android init` na raiz do projeto
   - Gera `src-tauri/gen/android/` com projeto Gradle completo
   - Configura build.gradle.kts com NDK e Cargo

2. Resolver conflitos de compilação:
   - `chromiumoxide` não compila para Android → criar feature flag ou cfg
   - `tauri-plugin-updater` → condicionalmente incluído

3. Configurar `src-tauri/Cargo.toml`:
   ```toml
   [target.'cfg(not(target_os = "android"))'.dependencies]
   chromiumoxide = "0.9"
   tauri-plugin-updater = "2.10"

   [target.'cfg(target_os = "android")'.dependencies]
   # dependências Android-specific aqui
   ```

4. Criar stub do browser crate para Android:
   ```rust
   // crates/browser/src/mod.rs
   #[cfg(not(target_os = "android"))]
   mod desktop;
   #[cfg(target_os = "android")]
   mod mobile;

   // mobile/mod.rs
   pub struct BrowserManager;
   impl BrowserManager {
       pub async fn bypass_cloudflare(&self, _url: &str) -> Result<()> {
           Err(HagitoriError::extension("Browser not available on Android"))
       }
   }
   ```

5. Primeira build: `pnpm tauri android dev`
   - Verificar que o app abre no emulador/device
   - Frontend React carrega no WebView do Tauri
   - HTTP requests funcionam (wreq compila para ARM)
   - SQLite funciona (rusqlite bundled)
   - Extensões JS funcionam (rquickjs compila para ARM)

**Resultado esperado**: App funcional no Android com todas as features exceto
browser automation e auto-update.

---

### Fase 2: Adaptação de Storage e Permissões

**Objetivo**: Garantir que paths de arquivo e permissões funcionem corretamente
no Android.

**Passos**:

1. Configurar `AndroidManifest.xml`:
   ```xml
   <uses-sdk android:minSdkVersion="29" android:targetSdkVersion="34" />
   <uses-permission android:name="android.permission.INTERNET" />
   <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
   <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
   <uses-permission android:name="android.permission.FOREGROUND_SERVICE_DATA_SYNC" />
   ```

   > **Android 10+ (API 29)** garante: Scoped Storage nativo (sem `WRITE_EXTERNAL_STORAGE`),
   > WebView Chrome 80+ (TLS 1.3, Brotli, ES2020), `NetworkCallback` estável,
   > `requestedWithHeaderMode` disponível em API 33+ (fallback via intercept para API 29-32).

2. Adaptar paths no crate `config`:
   ```rust
   pub fn data_dir() -> Result<PathBuf> {
       #[cfg(target_os = "android")]
       {
           // Usar environment variable ou JNI para obter app data dir
           // Tauri fornece via tauri::api::path::app_data_dir()
       }
       #[cfg(not(target_os = "android"))]
       {
           // Lógica atual: dirs::config_dir() / "hagitori"
       }
   }
   ```

3. Implementar SAF (Storage Access Framework) picker para downloads:
   - Na primeira tentativa de download, mostrar picker de pasta
   - Salvar URI persistente no `config.db`
   - Usar `ContentResolver` para I/O em pastas externas

4. Configurar WAL mode para SQLite:
   ```rust
   conn.execute_batch("PRAGMA journal_mode=WAL;")?;
   ```

5. Testar persistência de dados entre reinstalações:
   - Dados internos (`/data/data/`) → só persistem com backup
   - Downloads em SAF pasta → persistem sempre

6. Configurar Auto Backup:
   ```xml
   <application android:allowBackup="true"
                android:fullBackupContent="@xml/backup_rules">
   ```

**Resultado esperado**: Storage funcional com paths corretos, SAF picker para
downloads, e backup automático configurado.

---

### Fase 3: UI Mobile

**Objetivo**: Adaptar o frontend para experiência mobile (bottom navigation,
layout responsivo, sem banner).

**Passos**:

1. Criar hook `usePlatform()`:
   ```typescript
   // hooks/usePlatform.ts
   import { platform } from '@tauri-apps/plugin-os';
   export function usePlatform() {
     const [isMobile, setIsMobile] = useState(false);
     useEffect(() => {
       platform().then(p => setIsMobile(p === 'android' || p === 'ios'));
     }, []);
     return { isMobile };
   }
   ```

2. Criar componente `BottomNav.tsx`:
   - 5 itens: Home, Search, Downloads, Extensions, Settings
   - Ícones + labels
   - Active state highlight
   - Safe area insets para notch/navigation bar

3. Modificar `Layout.tsx`:
   ```tsx
   export function Layout() {
     const { isMobile } = usePlatform();
     return isMobile ? <MobileLayout /> : <DesktopLayout />;
   }
   ```
   - **MobileLayout**: Content + BottomNav (sem sidebar, sem banner)
   - **DesktopLayout**: Sidebar + Content (layout atual)

4. Ajustes CSS responsivos:
   - Grid de mangás: 2-3 colunas no mobile vs 4-6 no desktop
   - Touch targets: mín 48px para botões
   - Font sizes: maiores para legibilidade
   - Bottom padding: para não cobrir o BottomNav

5. Remover/esconder `UpdateModal` no mobile (não tem auto-update desktop)

6. Adaptar `MangaDetail`: layout vertical no mobile (cover em cima, info embaixo)

**Resultado esperado**: UI mobile nativa com bottom navigation, sem sidebar/banner,
responsiva e touch-friendly.

---

### Fase 4: Plugin Browser Mobile (Cloudflare Bypass)

**Objetivo**: Reimplementar o browser crate para Android usando WebView nativo.

Esta é a fase mais complexa e pode ser dividida em sub-fases:

#### Fase 4.1: Plugin Tauri Básico

1. Criar plugin: `pnpm tauri plugin new browser-mobile --android`

2. Estrutura do plugin:
   ```
   plugins/browser-mobile/
   ├── Cargo.toml
   ├── build.rs
   ├── src/
   │   └── lib.rs          ← Rust bridge (definição de comandos)
   ├── android/
   │   ├── build.gradle.kts
   │   └── src/main/kotlin/com/hagitori/browser/
   │       ├── BrowserPlugin.kt     ← Plugin principal
   │       ├── WebViewManager.kt    ← Gerenciador de WebViews
   │       └── ChallengeeSolver.kt  ← Cloudflare solver
   └── permissions/
       └── default.toml
   ```

3. Implementar `BrowserPlugin.kt`:
   ```kotlin
   @TauriPlugin
   class BrowserPlugin(private val activity: Activity) : Plugin(activity) {

       private var webViewManager: WebViewManager? = null

       override fun load(webView: WebView) {
           webViewManager = WebViewManager(activity)
       }

       @Command
       fun solveCloudflare(invoke: Invoke) {
           val url = invoke.getString("url") ?: return invoke.reject("url required")
           webViewManager?.solveChallenge(url) { result ->
               invoke.resolve(result)
           }
       }

       @Command
       fun interceptRequest(invoke: Invoke) {
           // ... intercept via shouldInterceptRequest
       }
   }
   ```

#### Fase 4.2: WebView Manager

1. Implementar `WebViewManager.kt`:
   ```kotlin
   class WebViewManager(private val activity: Activity) {
       private var webView: WebView? = null

       fun createWebView(): WebView {
           return WebView(activity).apply {
               settings.apply {
                   javaScriptEnabled = true
                   domStorageEnabled = true
                   // Custom UA sem "wv"
                   userAgentString = buildChromeUserAgent()
                   // Android 13+ (API 33): remover X-Requested-With via API nativa
                   // Android 10-12 (API 29-32): remover via shouldInterceptRequest
                   if (Build.VERSION.SDK_INT >= 33) {
                       requestedWithHeaderMode = OMIT_HEADER
                   }
               }
               webViewClient = InterceptingWebViewClient()
               addJavascriptInterface(JsBridge(), "HagitoriBridge")
           }
       }
   }
   ```

2. Configuração anti-detecção:
   - User-Agent: Chrome mobile sem "wv" suffix
   - Remover `X-Requested-With` header
   - JavaScript injection para esconder `navigator.webdriver`

#### Fase 4.3: Cloudflare Solver no WebView

1. Implementar `ChallengeSolver.kt`:
   - Navegar para URL alvo via `webView.loadUrl()`
   - Detectar challenge via `evaluateJavascript()`:
     ```javascript
     document.title.includes("Just a moment") ||
     document.querySelector("#turnstile-wrapper") !== null
     ```
   - Simular toque no Turnstile via `MotionEvent`:
     ```kotlin
     // Injetar JS para obter posição do iframe
     webView.evaluateJavascript("""
         var iframe = document.querySelector('iframe[src*="challenges.cloudflare"]');
         var rect = iframe.getBoundingClientRect();
         HagitoriBridge.onTurnstilePosition(rect.x + rect.width/2, rect.y + rect.height/2);
     """)

     // Despachar evento de toque
     val downEvent = MotionEvent.obtain(downTime, eventTime, ACTION_DOWN, x, y, 0)
     webView.dispatchTouchEvent(downEvent)
     // ... delay ... ACTION_UP
     ```
   - Extrair cookies via `CookieManager.getInstance().getCookie(url)`
   - Retornar cookies para Rust via plugin callback

#### Fase 4.4: Interceptação de Requests

1. Implementar `InterceptingWebViewClient`:
   ```kotlin
   class InterceptingWebViewClient(
       private val onBodyCaptured: (String, ByteArray) -> Unit
   ) : WebViewClient() {

       override fun shouldInterceptRequest(
           view: WebView, request: WebResourceRequest
       ): WebResourceResponse? {
           if (shouldCapture(request.url.toString())) {
               val cookies = CookieManager.getInstance().getCookie(request.url.toString())
               val okRequest = Request.Builder()
                   .url(request.url.toString())
                   .headers(request.requestHeaders.toHeaders())
                   .apply { if (cookies != null) header("Cookie", cookies) }
                   .build()

               val response = okHttpClient.newCall(okRequest).execute()
               val body = response.body?.bytes() ?: ByteArray(0)
               onBodyCaptured(request.url.toString(), body)

               return WebResourceResponse(
                   response.header("content-type"),
                   null,
                   response.code,
                   response.message,
                   response.headers.toMap(),
                   ByteArrayInputStream(body)
               )
           }
           return null
       }
   }
   ```

2. Para interceptar POST body, injetar JS que captura fetch/XMLHttpRequest:
   ```javascript
   (function() {
       const originalFetch = window.fetch;
       window.fetch = function(url, options) {
           if (options?.body) {
               HagitoriBridge.onPostBody(url, options.body, options.method || 'POST');
           }
           return originalFetch.apply(this, arguments);
       };
   })();
   ```

**Resultado esperado**: Cloudflare bypass funcional no Android via WebView,
com interceptação de requests e responses para download via browser.

---

### Fase 5: Updater Mobile

**Objetivo**: Implementar sistema de atualização para Android.

**Opções**:

#### Opção A: Play Store (Recomendado para produção)

- Sem código de update necessário
- In-App Update API apenas para prompts de atualização
- Dependency: `com.google.android.play:app-update`

#### Opção Escolhida: Self-hosted (GitHub Releases) 

1. Criar plugin Tauri ou usar Kotlin direto:
   ```kotlin
   @Command
   fun checkForUpdate(invoke: Invoke) {
       val currentVersion = BuildConfig.VERSION_NAME
       // GET https://github.com/hagitori/hagitori/releases/latest
       // Parse latest version from response
       invoke.resolve(JSObject().apply {
           put("currentVersion", currentVersion)
           put("latestVersion", latestVersion)
           put("updateAvailable", latestVersion > currentVersion)
           put("downloadUrl", apkUrl)
       })
   }

   @Command
   fun downloadAndInstallUpdate(invoke: Invoke) {
       val url = invoke.getString("url")
       // Download APK → FileProvider URI
       // Intent(ACTION_INSTALL_PACKAGE)
       // Requer: REQUEST_INSTALL_PACKAGES permission
   }
   ```

2. Adicionar permissão:
   ```xml
   <uses-permission android:name="android.permission.REQUEST_INSTALL_PACKAGES" />
   ```

**Resultado esperado**: Mecanismo de atualização funcional, seja via Play Store
ou self-hosted.

---

### Fase 6: Download em Background

**Objetivo**: Downloads continuam mesmo com o app em background.

1. Implementar Foreground Service para downloads:
   ```kotlin
   class DownloadService : Service() {
       override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
           val notification = NotificationCompat.Builder(this, CHANNEL_ID)
               .setContentTitle("Downloading manga...")
               .setSmallIcon(R.drawable.ic_download)
               .build()
           startForeground(NOTIFICATION_ID, notification)
           return START_NOT_STICKY
       }
   }
   ```

2. Criar Notification Channel:
   ```kotlin
   val channel = NotificationChannel(
       "downloads",
       "Downloads",
       NotificationManager.IMPORTANCE_LOW
   )
   notificationManager.createNotificationChannel(channel)
   ```

3. Atualizar progresso na notificação

**Resultado esperado**: Downloads não são interrompidos quando o app vai para background.

---

### Fase 7: Polishing e Features Adicionais

**Objetivo**: Finalização e features extras.

1. **Notificações**: Avisar quando download completa
2. **Deep links**: Abrir URLs de mangá diretamente no app
3. **Share target**: Receber URLs compartilhadas de outros apps
4. **Splash screen**: Tela de carregamento nativa
5. **Dark/Light theme**: Respeitar tema do sistema Android
6. **Haptic feedback**: Vibração em ações touch

---

## 11. Adaptações Específicas para Android

### Versão Mínima: Android 10 (API 29)

**Justificativa**:
- **Scoped Storage**: API 29 introduziu Scoped Storage — não precisa de `WRITE_EXTERNAL_STORAGE`
- **WebView**: Chrome 80+ garantido no system WebView (suporta ES2020, TLS 1.3, Brotli)
- **Network**: `ConnectivityManager.NetworkCallback` estável
- **Foreground Service**: Tipos de foreground service (`dataSync`) disponíveis
- **Market share**: ~95% dos dispositivos Android ativos (2024)
- **Tauri 2 suporte**: Android 7+ (API 24), mas API 29 é o mínimo recomendado

**Configuração no Gradle** (`build.gradle.kts` do plugin Tauri):
```kotlin
android {
    compileSdk = 34
    defaultConfig {
        minSdk = 29      // Android 10
        targetSdk = 34   // Android 14
    }
}
```

### Limites de Recursos

```rust
// crates/extensions/src/extension.rs
#[cfg(target_os = "android")]
const MAX_WORKERS: usize = 3;      // vs 5 no desktop
#[cfg(not(target_os = "android"))]
const MAX_WORKERS: usize = 5;

// Download engine: concurrent page downloads
#[cfg(target_os = "android")]
const MAX_CONCURRENT_PAGES: usize = 2;  // vs 3-5 no desktop
#[cfg(not(target_os = "android"))]
const MAX_CONCURRENT_PAGES: usize = 5;
```

### Lifecycle Management

Android pode matar o app a qualquer momento. Precisa garantir:

1. **Session persistence**: Salvar sessões HTTP no `onPause` (não só no `Exit`)
   ```kotlin
   override fun onPause() {
       // Trigger Rust to persist sessions
       invoke("persist_sessions")
   }
   ```

2. **Download state**: Usar `DownloadHistory` SQLite para retomar downloads
   após app ser morto

3. **WebView cleanup**: Destruir WebViews quando não necessárias para liberar memória

### Network Handling

```kotlin
// Detectar mudanças de rede
val connectivityManager = getSystemService(ConnectivityManager::class.java)
connectivityManager.registerDefaultNetworkCallback(object : ConnectivityManager.NetworkCallback() {
    override fun onLost(network: Network) {
        // Pausar downloads, notificar frontend
    }
    override fun onAvailable(network: Network) {
        // Retomar downloads
    }
})
```

### WebView vs Chrome no Android

| Aspecto | Chrome Desktop (CDP) | Android WebView |
|---------|---------------------|-----------------|
| Controle | Total (CDP protocol) | Parcial (Java API) |
| User Agent | Configurável | Configurável (`setUserAgentString`) |
| Headers extras | CDP `setExtraHTTPHeaders` | Solo via `loadUrl(url, headers)` |
| `X-Requested-With` | N/A | API 33+: `OMIT_HEADER`. API 29-32: remoção via `shouldInterceptRequest` |
| Response body | CDP `getResponseBody` | Via `shouldInterceptRequest` workaround |
| POST body | CDP `requestWillBeSent` | Via JS injection (`fetch`/`XHR` hook) |
| Mouse/Touch | CDP `dispatchMouseEvent` | `MotionEvent` injection |
| Headless | Sim | Não (sempre renderiza) |
| Cookie access | CDP `getCookies` | `CookieManager.getCookie()` |
| JS execution | CDP `Runtime.evaluate` | `evaluateJavascript()` |
| Multiple tabs | Multiple pages | Multiple WebView instances |
| TLS fingerprint | Chrome nativo | System WebView (diferente do Chrome). API 29+ garante TLS 1.3 |

### Persistência de Dados

| Cenário | Dados preservados? |
|---------|-------------------|
| App update (Play Store) | ✅ Sim |
| App update (APK manual, mesmo package) | ✅ Sim |
| Desinstalar + reinstalar | ❌ Não (exceto SAF downloads e Auto Backup) |
| Clear App Data | ❌ Não |
| Factory reset | ❌ Não |

**Proteção via Auto Backup**:
```xml
<full-backup-content>
    <include domain="database" path="." />      <!-- 5 SQLite databases -->
    <include domain="sharedpref" path="." />     <!-- localStorage do WebView -->
    <exclude domain="database" path="sessions.db" />  <!-- sessões são temporárias -->
    <exclude domain="file" path="browser_profile/" /> <!-- perfil do browser é temporário -->
</full-backup-content>
```

**Downloads via SAF**: Salvos fora do app data → **sobrevivem desinstalação**.

---

## 12. Features Futuras (Reader, Search, Local Files)

### Reader InApp

**Objetivo**: Ler capítulos baixados diretamente no app, sem precisar abrir em reader externo.

**Frontend**:
```
/reader/:mangaId/:chapterNumber → ReaderPage.tsx
├── Page viewer (vertical scroll ou horizontal swipe)
├── Page preloading (carregar próximas 3 páginas)
├── Zoom controls (pinch-to-zoom no mobile)
├── Reading direction (LTR / RTL / vertical)
├── Progress tracking (capítulo lido / não lido)
└── Chapter navigation (próximo / anterior)
```

**Backend** (novo comando Tauri):
```rust
#[tauri::command]
async fn get_chapter_pages(
    manga_name: String,
    chapter_number: String,
    state: tauri::State<'_, AppState>
) -> Result<Vec<String>> {
    // Listar imagens na pasta do capítulo
    // Retornar como asset:// URLs para o WebView renderizar
}
```

**Protocolo de assets**: Usar `asset://` do Tauri para servir imagens locais ao WebView.

### Busca por Nome

**Objetivo**: Buscar mangás pelo nome em todas as extensões instaladas, não só por URL.

**Frontend**:
```
/search → SearchPage.tsx
├── Input de busca com debounce (300ms)
├── Seletor de extensões (qual usar para buscar)
├── Grid de resultados com cards de mangá
└── Infinite scroll / pagination
```

**Backend**:
```rust
// Nova trait method em MangaProvider
#[async_trait]
pub trait MangaProvider: Send + Sync {
    // ... existing methods ...
    async fn search(&self, query: &str) -> Result<Vec<Manga>>;
}

// Novo comando Tauri
#[tauri::command]
async fn search_manga(
    query: String,
    extension_id: Option<String>,
    state: tauri::State<'_, AppState>
) -> Result<Vec<SearchResult>> {
    // Se extension_id fornecido: buscar em uma extensão específica
    // Senão: buscar em todas as extensões (fan-out)
}
```

**Extension API** (nova função JS):
```javascript
class Extension {
    async search(query) {
        const resp = await fetch(`${this.baseUrl}/search?q=${encodeURIComponent(query)}`);
        const results = await resp.json();
        return results.map(r => new Manga({ id: r.url, name: r.title, cover: r.image }));
    }
}
```

### Suporte a Arquivos Locais

**Objetivo**: Importar e ler mangás/comics de arquivos locais (CBZ, ZIP, pastas de imagens).

**Frontend**:
```
/local → LocalFilesPage.tsx
├── File/folder browser (ou SAF picker no Android)
├── Scan de diretório para CBZ/ZIP/pastas
├── Importar para biblioteca
└── Ler diretamente
```

**Backend**:
```rust
#[tauri::command]
async fn scan_local_files(
    path: String,
    state: tauri::State<'_, AppState>
) -> Result<Vec<LocalManga>> {
    // Recursivamente buscar em 'path':
    // - .cbz / .zip files → extrair metadata (ComicInfo.xml se existir)
    // - Pastas com imagens → tratar como capítulo
    // Retornar lista de mangás locais encontrados
}

#[tauri::command]
async fn import_local_manga(
    path: String,
    state: tauri::State<'_, AppState>
) -> Result<()> {
    // Adicionar à biblioteca como manga local
    // source = "local"
}
```

### Priorização de Features

| Feature | Impacto | Esforço | Prioridade |
|---------|---------|---------|------------|
| Reader InApp | Alto (UX principal) | Médio | 🔴 Alta |
| Busca por Nome | Alto (usabilidade) | Médio | 🔴 Alta |
| Arquivos Locais | Médio (nicho) | Baixo | 🟡 Média |

---

## 13. Referência de Comandos Tauri

### Manga (5 comandos)

| Comando | Parâmetros | Retorno |
|---------|-----------|---------|
| `get_manga` | `url: String` | `Manga` |
| `get_chapters` | `manga_id: String` | `Vec<Chapter>` |
| `get_details` | `manga_id: String` | `MangaDetails` |
| `set_extension_lang` | `extension_id: String, lang: String` | `()` |
| `list_extensions` | — | `Vec<ExtensionMeta>` |

### Download (2 comandos)

| Comando | Parâmetros | Retorno |
|---------|-----------|---------|
| `download_chapters` | `items: Vec<DownloadRequest>` | `()` (progresso via events) |
| `cancel_download` | — | `()` |

### Config (3 comandos)

| Comando | Parâmetros | Retorno |
|---------|-----------|---------|
| `get_config` | `key: String` | `Option<String>` |
| `set_config` | `key: String, value: String` | `()` |
| `get_download_path` | — | `String` |

### Library (11 comandos)

| Comando | Parâmetros | Retorno |
|---------|-----------|---------|
| `library_list` | — | `Vec<LibraryEntry>` |
| `library_get` | `manga_id: String` | `Option<LibraryEntry>` |
| `library_add` | `manga: Manga` | `()` |
| `library_remove` | `manga_id: String` | `()` |
| `library_update_chapters` | `manga_id: String, chapters: Vec<Chapter>` | `()` |
| `library_update_details` | `manga_id: String, details: MangaDetails` | `()` |
| `library_update_cover` | `manga_id: String, cover_url: String` | `()` |
| `library_set_source_meta` | `source: String, display_name: String` | `()` |
| `library_get_source_meta` | — | `HashMap<String, String>` |
| `library_set_extension_lang` | `extension_id: String, langs: Vec<String>` | `()` |
| `library_get_extension_langs` | — | `HashMap<String, Vec<String>>` |

### Sync / Catálogo (9 comandos)

| Comando | Parâmetros | Retorno |
|---------|-----------|---------|
| `fetch_catalog` | — | `Vec<CatalogEntry>` |
| `check_extension_updates` | — | `Vec<UpdateInfo>` |
| `install_catalog_extension` | `entry: CatalogEntry` | `()` |
| `update_catalog_extension` | `entry: CatalogEntry` | `()` |
| `remove_catalog_extension` | `extension_id: String` | `()` |
| `list_installed_extensions` | — | `Vec<InstalledExtension>` |
| `set_extension_auto_update` | `extension_id: String, enabled: bool` | `()` |
| `set_catalog_url` | `url: String` | `()` |
| `auto_update_extensions` | — | `AutoUpdateResult` |

---

## 14. Referência de Crates Rust

### hagitori-core

**Entidades**:
- `Manga { id, name, cover, source, url }`
- `Chapter { id, number, name, title, date, scanlator }`
- `Pages { chapter_id, chapter_number, manga_name, pages, headers, use_browser, scanlator }`
- `MangaDetails { id, name, synopsis, author, artist, tags, alt_titles, status, source }`
- `ExtensionMeta { id, name, lang, version, version_id, domains, languages, features, icon }`

**Traits**:
- `MangaProvider { meta(), get_manga(), get_chapters(), get_pages(), get_details(), set_lang() }`

**Error**:
- `HagitoriError { Extension(String), Config(String), Http(String), Download(String), ... }`
- Implementa `From<T>` para conversão automática de erros

### hagitori-http

**Structs**:
- `HttpClient { client: wreq::Client, session_store: DomainSessionStore }`
  - `new()` → Chrome145 TLS emulation, 30s timeout, 10s connect
  - `get(url, opts)`, `get_text(url, opts)`, `get_bytes(url, opts)`
  - `post(url, body, opts)`, `post_form(url, form, opts)`, `post_empty(url, opts)`
  - `session_store()` → access in-memory session store
- `DomainSessionStore { sessions: RwLock<HashMap<String, DomainSession>> }`
  - `get(domain)`, `set(domain, session)`, `import_all()`, `export_all()`
- `DomainSession { cookies, headers, user_agent }`
- `RequestOptions { headers, timeout, referer }`

### hagitori-browser

**Structs**:
- `BrowserManager` — gerencia instância Chrome
  - `launch()` → inicia Chrome com stealth config
  - `new_page()` → cria nova aba
  - `close()` → fecha browser

**Modules**:
- `chrome.rs` — detecção do Chrome no sistema
- `stealth.rs` — configuração anti-detecção (UA, window size, flags)
- `cloudflare.rs` — solver de challenges (Turnstile click, cookie extract)
- `intercept.rs` — listener CDP para request/response interception

### hagitori-extensions

**Structs**:
- `JsRuntime { data: Arc<RuntimeData> }` — factory de runtime data
- `RuntimeData { http_client, browser_manager }` — dados compartilhados
- `JsWorker { tx: mpsc::UnboundedSender }` — worker com canal de comunicação
- `JsExtension` — implementa `MangaProvider` via QuickJS
  - Worker pool (max 5), circuit breaker (max 5 falhas)
  - `call_js_function(name, args)` → gerencia workers

**APIs registradas** (ver seção 6 para referência completa)

**Loader** (`loader.rs`):
- Escaneia diretório de extensões recursivamente (max depth 4)
- Para cada diretório com `package.json`:
  - Parse manifest como `ExtensionManifest`
  - Lê `index.js` como script
  - Lê `icon.png` como base64 (opcional)
  - Cria `JsExtension` com `JsRuntime`

### hagitori-providers

**Struct**:
- `ProviderRegistry { providers: Vec<Arc<dyn MangaProvider>> }`
  - `load_extensions(dir, http_client, browser_manager)` → scan + load
  - `find_by_url(url)` → match domain against providers
  - `list()` → all loaded providers
  - `get(id)` → specific provider

### hagitori-download

**Engine** (`engine.rs`):
- `download_chapter()` — orquestra o download de um capítulo
  - Dual-path: HTTP direto ou via browser automation
  - Concurrency: `Semaphore` para limitar downloads paralelos
  - Retry: 3 tentativas com backoff exponencial
  - Rate limiting: respeita `Retry-After` header (HTTP 429)
  - Emite `download-progress` events para o frontend

**Image** (`image.rs`):
- Extração de extensão de arquivo a partir da URL
- Resolução de filename de saída (001.png, 002.jpg, etc.)
- Conversão de formato de imagem (PNG/JPEG/WebP) via `image` crate

### hagitori-config

**Managers** (5 databases):
- `ConfigManager` — settings key/value (`config.db`)
- `ExtensionRegistry` — extensões instaladas (`extensions.db`)
- `LibraryManager` — mangás, capítulos, detalhes (`library.db`)
- `SessionStore` — cookies/headers persistidos (`sessions.db`)
- `DownloadHistory` — histórico de downloads (`history.db`)

Cada manager:
- `new(data_dir)` → cria/abre banco + roda migrations
- CRUD operations com prepared statements
- `rusqlite` com feature `bundled` (SQLite compilado no binário)

### hagitori-sync

**Structs**:
- `CatalogFetcher` — download + parse de `catalog.json`
- `ExtensionInstaller` — download atômico + validação SHA-256
- `UpdateChecker` — compara versões (installed vs catalog)
- `AutoUpdater` — roda `UpdateChecker` + `ExtensionInstaller` para todas

### hagitori-grouper

- `create_archive(chapter_dir, format)` → CBZ ou pasta
- `ComicInfo` struct → serializa para ComicInfo.xml
- Formato CBZ = ZIP com imagens + ComicInfo.xml

---

## 15. Referência do Frontend

### Páginas

| Página | Rota | Função |
|--------|------|--------|
| `Home.tsx` | `/` | Biblioteca: grid de mangás salvos, organizado por fonte |
| `Search.tsx` | `/search` | Busca por URL + resultados |
| `ExtensionManga.tsx` | `/library/:source` | Mangás de uma fonte específica |
| `MangaDetail.tsx` | `/manga/:id` | Detalhes + lista de capítulos + botão download |
| `Downloads.tsx` | `/downloads` | Fila de downloads + progresso |
| `Extensions.tsx` | `/extensions` | Tabs: Catálogo + Instaladas |
| `Settings.tsx` | `/settings` | Configurações (path, formato, idioma, catálogo) |

### Componentes UI

| Componente | Função |
|-----------|--------|
| `Layout.tsx` | Wrapper com Sidebar + Outlet |
| `Sidebar.tsx` | Navegação lateral com links + ícones |
| `Badge.tsx` | Badge numérico/texto |
| `Button.tsx` | Botão com variantes (primary, secondary, ghost, danger) |
| `Card.tsx` | Card container com header/body |
| `EmptyState.tsx` | Estado vazio com ícone + mensagem |
| `Input.tsx` | Input de texto estilizado |
| `ProgressBar.tsx` | Barra de progresso animada |
| `Select.tsx` | Dropdown select estilizado |
| `Skeleton.tsx` | Loading placeholder animado |
| `Toast.tsx` | Notificação toast (success/error/info) |
| `Toggle.tsx` | Switch toggle on/off |
| `UpdateModal.tsx` | Modal de atualização do app |
| `AddRepoModal.tsx` | Modal para adicionar URL de catálogo |
| `CatalogTab.tsx` | Tab com extensões disponíveis para instalar |
| `InstalledTab.tsx` | Tab com extensões instaladas |
| `ChapterList.tsx` | Lista de capítulos com seleção multiple |
| `MangaHeader.tsx` | Header com cover + info do mangá |

### i18n

Dois idiomas suportados:
- `en.ts` — English
- `pt-br.ts` — Português Brasileiro

Hook `useTranslation()` retorna `t(key)` function.

---

## 16. Constantes e Configurações Hardcoded

### Backend

```rust
// Extensions
const MAX_WORKERS: usize = 5;              // workers por extensão
const MAX_CONSECUTIVE_FAILURES: u32 = 5;   // circuit breaker
const CALL_TIMEOUT_MS: u64 = 30_000;       // 30s por method call
const INIT_TIMEOUT_MS: u64 = 60_000;       // 60s para inicialização
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;  // 64 MB por worker
const MAX_STACK_SIZE: usize = 2 * 1024 * 1024;  // 2 MB stack

// Cache
const CACHE_SIZE: usize = 256;             // LRU cache de manga/provider

// HTTP
const HTTP_TIMEOUT: Duration = 30s;         // timeout de request
const CONNECT_TIMEOUT: Duration = 10s;      // timeout de conexão
const POOL_MAX_IDLE: usize = 10;            // conexões idle por host

// Download
const MAX_RETRIES: u32 = 3;                // retentativas por página
const RATE_LIMIT_DEFAULT_WAIT: Duration = 5s;  // wait se Retry-After ausente

// Browser
const CF_TIMEOUT: Duration = 90s;           // timeout do Cloudflare solver
const CF_POLL_INTERVAL: Duration = 500ms;   // intervalo de polling
const BROWSER_WINDOW_SIZE: (u32, u32) = (1280, 720);

// Loader
const MAX_SCAN_DEPTH: usize = 4;            // profundidade de scan de extensões

// Installer
const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024;  // 5 MB por arquivo
const MAX_EXTENSION_SIZE: u64 = 20 * 1024 * 1024;  // 20 MB total
```

### Frontend

```typescript
// Query Client
const STALE_TIME = 5 * 60 * 1000;  // 5 minutos
const RETRY_COUNT = 1;

// Download
const MAX_RETRY_COUNT = 3;
const RETRY_DELAY_MS = 2000;

// UI
const DEBOUNCE_SEARCH_MS = 300;
const TOAST_DURATION_MS = 5000;
```

---

## Apêndice A: Arquivos Modificáveis por Plataforma

Lista de arquivos que precisam de `cfg(target_os)` ou adaptação para Android:

| Arquivo | Tipo de Adaptação |
|---------|------------------|
| `crates/browser/src/mod.rs` | `cfg(target_os)` switch desktop/mobile |
| `crates/browser/src/chrome.rs` | Desktop only (Windows/Linux paths) |
| `crates/browser/src/cloudflare.rs` | Desktop only (CDP-based) |
| `crates/browser/src/stealth.rs` | Desktop only |
| `crates/browser/src/intercept.rs` | Desktop only |
| `crates/extensions/src/extension.rs` | `MAX_WORKERS` conditional |
| `crates/config/src/database.rs` | `data_dir()` path resolution |
| `src/lib.rs` | conditional plugin inclusion |
| `src/components/ui/Layout.tsx` | Platform-conditional rendering |
| `src/components/ui/UpdateModal.tsx` | Desktop only |
| `tauri.conf.json` | Mobile overrides |

## Apêndice B: Dependências Exclusivas Desktop

Estas dependências são usadas apenas no desktop e devem ser condicionalizadas:

```toml
[target.'cfg(not(target_os = "android"))'.dependencies]
chromiumoxide = "0.9"
tauri-plugin-updater = "2.10"
```

## Apêndice C: Checklist de Migração Android

- [ ] `pnpm tauri android init` (configurar `minSdk = 29` no Gradle)
- [ ] Condicionalizar `chromiumoxide` com `cfg(target_os)`
- [ ] Condicionalizar `tauri-plugin-updater` com `cfg(target_os)`
- [ ] Criar stub/mobile do browser crate
- [ ] Primeira build Android (`pnpm tauri android dev`)
- [ ] Verificar HTTP/SQLite/QuickJS funcionam no device
- [ ] Configurar `AndroidManifest.xml` (permissões)
- [ ] Adaptar `data_dir()` para Android paths
- [ ] Implementar SAF picker para downloads
- [ ] Configurar Auto Backup (`backup_rules.xml`)
- [ ] Criar `BottomNav.tsx` component
- [ ] Criar `usePlatform()` hook
- [ ] Adaptar `Layout.tsx` para mobile/desktop
- [ ] Ajustes CSS responsivos (grid, touch targets)
- [ ] Criar plugin `browser-mobile` (Tauri plugin)
- [ ] Implementar `WebViewManager.kt`
- [ ] Implementar `ChallengeSolver.kt` (Cloudflare)
- [ ] Implementar `InterceptingWebViewClient`
- [ ] Implementar JS injection para POST body capture
- [ ] Integrar plugin Kotlin com browser crate (JNI bridge)
- [ ] Implementar updater mobile (GitHub Releases self-hosted)
- [ ] Implementar Foreground Service para downloads
- [ ] Implementar Notification Channel
- [ ] Testar em dispositivos reais (ARM64)
- [ ] Testar com extensões reais (CF bypass)
- [ ] Configurar CI/CD para build Android
- [ ] Signing key para release APK
- [ ] Play Store listing (se aplicável)

---

> **Nota**: Este documento deve ser atualizado conforme a implementação avança.
> Cada fase completada deve ser marcada no checklist e quaisquer decisões
> arquiteturais divergentes devem ser documentadas aqui.
