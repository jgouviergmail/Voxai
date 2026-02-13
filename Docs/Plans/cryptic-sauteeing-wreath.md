# Plan: Commande de build unique CPU + CUDA

## Contexte
L'utilisateur veut une commande unique qui produit les deux distributions (CPU et CUDA) sous forme d'installeurs NSIS renommés, dans un dossier `dist/` facile à trouver. Actuellement, `build:windows` et `build:windows:cuda` sont deux commandes séparées, et le renommage des installeurs est manuel.

De plus, `prepare-worker.mjs` est maintenant partiellement redondant avec le `build.rs` qui compile automatiquement le worker avec le bon feature flag.

## Fichiers à modifier

### 1. `scripts/build-all.mjs` (NOUVEAU)
Script Node.js qui enchaîne les deux builds :
- Auto-détection CUDA_PATH (réutilise la logique de `prepare-worker.mjs`)
- Build CPU : `npx tauri build` → renomme installeur en `Voxai_<ver>_CPU_x64-setup.exe`
- Build CUDA : `npx tauri build --features cuda` → renomme installeur en `Voxai_<ver>_CUDA_x64-setup.exe`
- Output dans `src-tauri/target/release/dist/`
- Affiche un résumé avec tailles de fichiers
- Flags optionnels : `--cpu-only`, `--cuda-only` pour ne builder qu'une version

### 2. `scripts/prepare-worker.mjs` (SIMPLIFIER)
Le `build.rs` compile déjà le worker avec le bon feature flag. Simplifier `prepare-worker.mjs` :
- Supprimer la compilation du worker (plus besoin, build.rs le fait)
- Garder uniquement l'auto-détection CUDA_PATH et le lancement de `tauri build`
- Renommer en simple helper appelé par `build-all.mjs`

### 3. `package.json` (MODIFIER)
Ajouter le script :
```json
"build:all": "node scripts/build-all.mjs"
```
Simplifier les scripts existants pour refléter que build.rs gère le worker :
```json
"build:windows": "npx tauri build",
"build:windows:cuda": "npx tauri build --features cuda"
```
(plus besoin de `npm run build:worker` avant, build.rs le fait)

### 4. `build.rs` — pas de modification
Déjà correct après les fixes précédents.

## Architecture du script `build-all.mjs`

```
npm run build:all
│
├─ Phase 1: Validation
│   ├─ Vérifier que cargo, npx, tsc sont disponibles
│   └─ Auto-détecter CUDA_PATH (si build CUDA)
│
├─ Phase 2: Build CPU (sauf --cuda-only)
│   ├─ execSync("npx tauri build")
│   │   └─ build.rs compile worker SANS cuda automatiquement
│   ├─ Copier installeur → dist/Voxai_<ver>_CPU_x64-setup.exe
│   └─ Log taille
│
├─ Phase 3: Build CUDA (sauf --cpu-only)
│   ├─ execSync("npx tauri build --features cuda")
│   │   └─ build.rs compile worker AVEC cuda automatiquement
│   ├─ Copier installeur → dist/Voxai_<ver>_CUDA_x64-setup.exe
│   └─ Log taille
│
└─ Phase 4: Résumé
    └─ Afficher les fichiers produits avec tailles
```

## Output attendu

```
src-tauri/target/release/dist/
├── Voxai_0.1.1_CPU_x64-setup.exe    (~5 MB)
└── Voxai_0.1.1_CUDA_x64-setup.exe   (~150 MB)
```

## Vérification
1. `npm run build:all` → doit produire les 2 installeurs dans `dist/`
2. `npm run build:all -- --cpu-only` → un seul installeur CPU
3. `npm run build:all -- --cuda-only` → un seul installeur CUDA
4. Les scripts existants `npm run build:windows` et `npm run build:windows:cuda` continuent de fonctionner
5. Installer CPU → le worker log `[llm-worker] Model loaded (CPU mode)`
6. Installer CUDA → le worker log `[llm-worker] Model loaded with 99 GPU layers (CUDA ENABLED)`
