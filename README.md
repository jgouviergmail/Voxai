# Voxai

**Voxai** est une application de dictee vocale pour Windows qui transforme la parole en texte, l'enrichit par intelligence artificielle, puis l'injecte directement dans l'application active. Tout le traitement s'effectue localement sur votre machine, sans envoyer de donnees dans le cloud.

Le principe est simple : maintenez un raccourci clavier, parlez, relachez. Voxai transcrit votre voix via Whisper (whisper.cpp), applique un pipeline de post-traitement configurable (capitalisation, espacement, reformulation par LLM, traduction, substitutions), puis colle le texte final dans n'importe quelle application (editeur, navigateur, messagerie, etc.) comme si vous l'aviez tape au clavier.

---

## Presentation des fonctionnalites

### Dictee vocale (Push-to-talk)

Voxai utilise un systeme de **push-to-talk** avec un raccourci clavier global (par defaut `Ctrl+Shift+Espace`). Tant que vous maintenez le raccourci, l'application enregistre votre voix via le microphone selectionne. Des que vous relachez, la transcription demarre automatiquement. Ce mode evite les faux declenchements et vous donne un controle total sur ce qui est capture. Le raccourci est entierement configurable (touche + modificateurs).

### Transcription vocale (Whisper STT)

La reconnaissance vocale s'appuie sur **whisper.cpp**, une implementation performante du modele Whisper d'OpenAI. Voxai propose 4 modeles au format GGUF, du plus rapide au plus precis :

| Modele | Taille | Vitesse | Precision |
|--------|--------|---------|-----------|
| **Base** | 142 Mo | ~1s | Bonne pour la dictee courte |
| **Small** | 466 Mo | ~2-3s | Meilleur compromis vitesse/precision |
| **Medium** | 1,5 Go | ~5-8s | Haute precision |
| **Large v3** | 3,1 Go | ~10-15s | Meilleure precision (4 Go+ RAM) |

Les modeles se telechargent directement depuis l'interface dans l'onglet **Moteurs**. 8 langues sont supportees (francais, anglais, espagnol, allemand, italien, portugais, japonais, chinois) plus un mode de detection automatique.

### Pipeline de post-traitement

Apres la transcription, le texte passe par un pipeline en 5 etapes, chacune activable/desactivable independamment :

1. **Capitalisation** — Met en majuscule la premiere lettre de chaque phrase
2. **Espacement intelligent** — Corrige les espaces autour de la ponctuation
3. **Reformulation (LLM)** — Reecrit le texte selon un style choisi via un modele de langage
4. **Traduction (LLM)** — Traduit le texte dans une langue cible (40+ langues supportees)
5. **Substitutions** — Applique des regles de remplacement personnalisees (toujours en dernier, jamais ecrasees par le LLM)

Les etapes 3 et 4 necessitent un backend LLM actif (Ollama ou Local).

### Reformulation intelligente

La reformulation utilise un LLM pour reecrire le texte transcrit selon 6 styles integres :

- **Cleaned** — Corrige la grammaire et reconstruit les phrases mal formees typiques de la dictee
- **Professional** — Ton formel, vocabulaire soigne, adapte aux emails et documents officiels
- **Casual** — Ton decontracte et conversationnel, comme si vous parliez a un ami
- **Concise** — Supprime le superflu, fusionne les phrases, va a l'essentiel
- **Simplified** — Langage simple et accessible, phrases courtes
- **Structured** — Reorganise en paragraphes et listes a puces

Vous pouvez aussi **creer vos propres styles** avec des prompts personnalises (systeme + instruction), **modifier les prompts des styles integres** sans perdre les valeurs par defaut, et **reinitialiser** un prompt modifie a sa version d'origine.

Le pipeline integre un nettoyage automatique des artefacts LLM : suppression des balises `<think>` (mode reflexion Qwen3), des guillemets d'encadrement, et des preambules inutiles ("Voici le texte corrige :").

### Double backend LLM

Voxai supporte deux backends LLM au choix :

**Ollama** — Connecte-vous a un serveur Ollama local ou distant (par defaut `localhost:11434`, modele Mistral). Ideal si vous avez deja Ollama installe ou si vous preferez gerer vos modeles separement.

**Local (llama.cpp)** — 5 modeles quantizes Q4 embarques, telechargeables depuis l'interface :

| Modele | Taille | Langues | Particularite |
|--------|--------|---------|---------------|
| **Gemma 3 1B** (Q4) | 806 Mo | 140+ | Le plus rapide, contexte 128K |
| **Qwen3 1.7B** (Q4) | 1,1 Go | 100+ | Mode reflexion, multilingue |
| **Phi-4 Mini** (Q4) | 2,5 Go | 23 | Raisonnement avance (Microsoft) |
| **Qwen3 4B** (Q4) | 2,5 Go | 100+ | Meilleur polyvalent |
| **Gemma 3 4B** (Q4) | 2,5 Go | 140+ | Meilleur suivi d'instructions |

Le LLM local fonctionne via un **subprocess isole** (`voxai-llm-worker`) pour eviter un conflit de symboles entre whisper-rs et llama-cpp au niveau du linker (les deux embarquent des versions incompatibles de ggml).

### Hotkey de traitement de texte

En plus du push-to-talk, Voxai propose un **second raccourci optionnel** pour le traitement de texte sur selection. Selectionnez du texte dans n'importe quelle application, appuyez sur le raccourci : Voxai copie le texte, le reformule et/ou le traduit via le LLM, puis le recolle a la place de la selection. Pratique pour corriger ou traduire du texte deja ecrit.

### Gestion des modeles

Tous les modeles (Whisper et LLM) sont telechargeables directement depuis l'onglet **Moteurs** de l'interface :

- Telechargement en streaming depuis HuggingFace avec **barre de progression**
- **Annulation** possible a tout moment pendant le telechargement
- Detection et nettoyage automatique des fichiers `.downloading` incomplets
- Modeles stockes dans `%APPDATA%/Voxai/models/`
- Activation/desactivation/suppression depuis l'interface

### Acceleration GPU (NVIDIA CUDA)

Voxai propose un build optionnel avec support CUDA pour accelerer a la fois la transcription Whisper et l'inference LLM sur GPU NVIDIA. L'activation se fait via un toggle dans l'onglet General (desactive automatiquement si aucune carte NVIDIA n'est detectee).

### Fenetre overlay

Quand l'application est minimisee, une **fenetre flottante** (toujours au premier plan) affiche en temps reel l'etat de Voxai :

- **Pastille coloree** : vert (pret), rouge (enregistrement), jaune (traitement en cours)
- **Animation pulsante** pendant l'enregistrement et le traitement
- **Panneau deroulant** avec toggles rapides pour la reformulation (choix du style) et la traduction (choix de la langue cible)
- **Deplacable** par glisser-deposer
- Fermeture redirigee vers le tray (Alt+F4 ne la detruit pas)

### Injection de texte

Le texte final est **injecte dans l'application active** via simulation de Ctrl+V. Options :

- **Auto-entree** : appuie automatiquement sur Entree apres le collage
- **Restauration du presse-papiers** : sauvegarde et restaure le contenu precedent du presse-papiers apres injection

### Interface de configuration (5 onglets)

| Onglet | Contenu |
|--------|---------|
| **General** | Microphone, langue STT, raccourci push-to-talk, raccourci traitement texte, auto-entree, restauration presse-papiers, acceleration GPU, langue de l'interface |
| **Moteurs** | Telechargement/activation/suppression des modeles Whisper et LLM, indicateurs de statut (charge/non charge/telechargement) |
| **Post-traitement** | Backend LLM (Ollama/Local/Aucun), toggles capitalisation/espacement/reformulation/traduction, editeur de prompts avec reinitialisation, styles custom CRUD, zone de test (4 boutons : pipeline complet, reformulation, traduction, substitutions) |
| **Substitutions** | Regles de remplacement avec option case-sensitive, apercu en temps reel |
| **Historique** | 100 dernieres transcriptions avec texte original vs. final, horodatage |

### Internationalisation (i18n)

L'interface est disponible en 3 langues, selectionnables dans l'onglet General :

- English
- Francais
- Chinois (Zhongwen)

### Barre d'etat et tray systeme

- **Barre d'etat** integree dans l'en-tete fixe de l'application (toujours visible)
- **Icone tray** avec menu contextuel (afficher/masquer la fenetre, quitter)
- L'application reste active en arriere-plan quand la fenetre est fermee

### Historique des transcriptions

Chaque transcription est enregistree avec :
- Texte brut (avant post-traitement)
- Texte final (apres pipeline complet)
- Horodatage
- Possibilite de tout effacer

Limite a 100 entrees (les plus anciennes sont supprimees automatiquement).

---

## Stack technique

### Framework desktop : Tauri 2

[Tauri](https://tauri.app/) est le framework desktop qui relie le frontend web au backend Rust. Il fournit :
- Le **WebView** natif du systeme (pas de Chromium embarque, binaire leger)
- Le systeme d'**IPC** (Inter-Process Communication) entre le frontend et le backend via des commandes `#[tauri::command]` invoquees par `invoke()` cote TypeScript
- La gestion des **fenetres** (settings + overlay), du **tray systeme**, des **evenements** globaux et du **bundling** (installeur NSIS pour Windows)
- Le plugin `tauri-plugin-shell` pour le lancement du subprocess LLM worker

### Frontend : Solid.js + TailwindCSS

| Technologie | Version | Role |
|-------------|---------|------|
| [Solid.js](https://www.solidjs.com/) | 1.9 | Framework UI reactif (signaux, stores, rendu fin sans Virtual DOM) |
| [TailwindCSS](https://tailwindcss.com/) | 4 | Framework CSS utility-first (via plugin Vite `@tailwindcss/vite`) |
| [TypeScript](https://www.typescriptlang.org/) | 5.7 | Typage statique du frontend |
| [Vite](https://vite.dev/) | 6 | Build tool + serveur de developpement avec HMR (Hot Module Replacement) sur `localhost:1420` |
| `vite-plugin-solid` | 2.11 | Integration Solid.js dans Vite (compilation JSX) |
| `@tauri-apps/api` | 2 | SDK TypeScript pour communiquer avec le backend Rust via IPC |

Le frontend utilise le pattern **signaux reactifs** de Solid.js (`createSignal`, `createMemo`, `For`, `Show`) pour une reactivite fine sans re-rendering inutile. L'interface comporte deux points d'entree : `index.html` (fenetre principale avec les 5 onglets) et `overlay.html` (fenetre flottante).

### Backend : Rust 2021

| Crate | Version | Role |
|-------|---------|------|
| `tauri` | 2 | Framework desktop, IPC, fenetres, tray, bundling |
| `whisper-rs` | 0.15 | Bindings Rust pour whisper.cpp (transcription STT, GGUF, support CUDA) |
| `ollama-rs` | 0.3 | Client HTTP pour l'API Ollama (generation LLM, streaming) |
| `llama-cpp-2` | 0.1 | Bindings Rust pour llama.cpp (inference LLM locale) — **dans le worker uniquement** |
| `cpal` | 0.17 | Capture audio cross-platform (enumeration des peripheriques, enregistrement PCM) |
| `rubato` | 0.14 | Resampling audio haute qualite (FFT, conversion vers 16kHz mono pour Whisper) |
| `rdev` | 0.5 | Hook clavier global bas niveau (ecoute keydown/keyup sans focus fenetre) |
| `enigo` | 0.6 | Simulation de frappes clavier (Ctrl+V pour injection texte) |
| `arboard` | 3 | Lecture/ecriture du presse-papiers systeme |
| `hf-hub` | 0.4 | Client HuggingFace Hub (resolution de fichiers, telechargement de modeles) |
| `reqwest` | 0.12 | Client HTTP async avec support streaming (progression des telechargements) |
| `tokio` | 1 | Runtime async (timers, timeouts LLM) |
| `serde` / `serde_json` | 1 | Serialisation/deserialisation JSON (config, IPC, protocole worker) |
| `thiserror` | 2 | Derive macro pour les types d'erreur (`AppError`) |
| `chrono` | 0.4 | Horodatage des transcriptions |
| `uuid` | 1 | Generation d'identifiants uniques (historique, prompts custom) |
| `async-trait` | 0.1 | Traits async (`LlmBackend`, `SttEngine`) |
| `log` + `env_logger` | 0.4 / 0.11 | Logging structure |
| `dirs` | 6 | Resolution des repertoires systeme (`%APPDATA%`, `~/.config`) |
| `futures-util` | 0.3 | Utilitaires pour streams async (telechargement en streaming) |

### Outils de build

| Outil | Role |
|-------|------|
| `cargo` | Compilateur et gestionnaire de paquets Rust |
| `npm` | Gestionnaire de paquets Node.js (frontend) |
| `vite` | Build frontend (TypeScript + Solid.js → JS bundle) |
| `tauri-cli` | CLI Tauri pour le dev (`tauri dev`) et le packaging (`tauri build`) |
| `tauri-build` | Build script Rust pour Tauri (generation de ressources) |
| NSIS | Generateur d'installeur Windows (.exe) |
| CMake + MSVC | Compilation des dependances natives C/C++ (whisper.cpp, llama.cpp) |
| `prepare-worker.mjs` | Script Node.js pour builder et copier le binaire `voxai-llm-worker.exe` |

---

## Prerequis

- [Node.js](https://nodejs.org/) v18+
- [Rust](https://rustup.rs/) stable
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + CMake)
- `libclang.dll` accessible via la variable d'environnement `LIBCLANG_PATH`
- [Ollama](https://ollama.com/) (optionnel, pour le backend LLM Ollama)
- [NVIDIA CUDA Toolkit](https://developer.nvidia.com/cuda-toolkit) v12+ (optionnel, pour l'acceleration GPU)

---

## Demarrage rapide

```bash
# Installer les dependances frontend
npm install

# Developpement avec hot reload
npm run dev:full

# Build production (CPU)
npm run build:windows

# Build production (NVIDIA GPU)
npm run build:windows:cuda
```

Au premier lancement, telecharger un modele Whisper depuis l'onglet **Moteurs** avant d'enregistrer.

### Commandes de build

| Commande | Description |
|----------|-------------|
| `npm run dev` | Serveur Vite seul (frontend) |
| `npm run dev:worker` | Build du worker LLM (debug) |
| `npm run dev:full` | Worker + Tauri dev avec hot reload |
| `npm run build:windows` | Build CPU complet (worker + installeur NSIS) |
| `npm run build:windows:cuda` | Build NVIDIA complet (worker + installeur NSIS) |
| `npm run build:worker` | Build du worker LLM (release, CPU) |
| `npm run build:worker:cuda` | Build du worker LLM (release, CUDA) |

### Deux distributions

- **Build CPU** — Aucune dependance CUDA, bundle leger (~15 Mo)
- **Build NVIDIA** — Lie CUDA, inclut les librairies GPU (~300 Mo+), necessite GPU + drivers NVIDIA

### Variables d'environnement (Windows)

```powershell
$env:LIBCLANG_PATH = "C:\chemin\vers\libclang"
$env:Path += ";C:\Users\<utilisateur>\.cargo\bin"
$env:Path += ";C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin"
# Pour les builds CUDA uniquement :
$env:CUDA_PATH = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9"
```

---

## Structure du projet

```
voxai/
├── src/                              # Frontend (Solid.js + TypeScript)
│   ├── App.tsx                       # Application principale (routeur 5 onglets)
│   ├── Overlay.tsx                   # Fenetre flottante overlay
│   ├── components/
│   │   ├── settings/                 # GeneralTab, EnginesTab, PostProcessingTab,
│   │   │                             #   SubstitutionsTab, HistoryTab
│   │   ├── layout/                   # PageShell (en-tete + barre de statut), TabBar
│   │   └── ui/                       # Button, Input, Select, Toggle, ProgressBar
│   ├── lib/
│   │   ├── commands.ts               # Wrappers IPC Tauri (invoke)
│   │   ├── events.ts                 # Ecouteurs d'evenements
│   │   ├── stores.ts                 # Stores reactifs Solid.js
│   │   ├── i18n.ts                   # Contexte de localisation
│   │   ├── constants.ts              # Noms des styles integres
│   │   └── translations/             # en.ts, fr.ts, zh.ts
│   └── types/                        # Interfaces TypeScript
│
├── src-tauri/                        # Backend (Rust + Tauri 2)
│   ├── src/
│   │   ├── audio/                    # Capture audio + resampling (cpal, rubato)
│   │   ├── stt/                      # Moteur Whisper
│   │   ├── llm/                      # Backends Ollama + Local LLM, templates de prompts
│   │   ├── models/                   # Registre, cache, telechargeur
│   │   ├── postprocessing/           # Pipeline : capitalise → espacement → reformule
│   │   │                             #   → traduit → substitue
│   │   ├── injection/                # Injection de texte (Windows, enigo)
│   │   ├── hotkey/                   # Hook clavier global (rdev)
│   │   ├── config/                   # Persistance des parametres (JSON)
│   │   ├── history/                  # Stockage de l'historique
│   │   ├── commands/                 # Handlers de commandes IPC Tauri
│   │   └── tray/                     # Icone et menu du tray systeme
│   │
│   └── llm-worker/                   # Subprocess LLM (llama-cpp-2)
│       ├── Cargo.toml
│       └── src/main.rs               # Protocole IPC JSON sur stdin/stdout
│
├── scripts/
│   └── prepare-worker.mjs            # Build et copie du binaire worker
│
├── overlay.html                      # Point d'entree de la fenetre overlay
├── package.json
├── vite.config.ts
└── src-tauri/Cargo.toml              # Workspace : crate principal + llm-worker
```

---

## Architecture

### Isolation du LLM (subprocess)

Les crates `whisper-rs-sys` et `llama-cpp-sys-2` embarquent toutes les deux statiquement `ggml.c` avec des ABIs incompatibles. Les lier dans un meme binaire provoque des erreurs du linker (MSVC LNK2005) ou des crashes au runtime.

**Solution** : Le processus principal utilise uniquement `whisper-rs` (STT). L'inference LLM s'execute dans un subprocess separe `voxai-llm-worker` qui utilise `llama-cpp-2`. La communication se fait par JSON ligne par ligne sur stdin/stdout.

### Pipeline de traitement

```
Transcription brute
  → 1. Capitalisation (premiere lettre de chaque phrase)
  → 2. Espacement intelligent (ponctuation)
  → 3. Reformulation (recriture LLM selon le style choisi)
  → 4. Traduction (LLM vers la langue cible)
  → 5. Substitutions (regles de remplacement — toujours en dernier)
```

### Flux d'enregistrement

1. L'utilisateur maintient le raccourci (push-to-talk)
2. Capture audio en continu (cpal, resampling 16kHz mono via rubato)
3. Relachement du raccourci → transcription Whisper
4. Execution du pipeline de post-traitement
5. Injection du texte final dans l'application active (Ctrl+V)
6. Restauration du presse-papiers (optionnel)
7. Sauvegarde dans l'historique

---

## Configuration

Les parametres sont stockes dans `~/.config/Voxai/config.json` et editables depuis l'interface.

---

## Licence

Tous droits reserves.
