# Voxai UI Redesign - Plan Final

## Contexte

L'interface actuelle est fonctionnelle mais utilitaire. Objectif : la transformer en une UI de qualite "premium"
inspiree de Linear/Raycast tout en restant sobre. Zero changement fonctionnel - seulement du visuel.

**Problemes actuels :**
- Palette `gray-*` froide et sans personnalite
- Sections separees par de simples `border-t` (pas de groupement visuel)
- Headers de section tous identiques (monotone)
- Tabs basiques avec underline
- Overlay basique (blur faible, pas d'animation)
- Pas de micro-interactions (hover, focus, transitions)
- Emojis pour le theme toggle au lieu d'icones propres

**Direction design :** Style "Linear" - monochrome elegant, un accent bleu chirurgical, hierarchie par
opacite, surfaces par elevation, micro-interactions subtiles, glassmorphism raffine pour l'overlay.

---

## 1. Design System - `src/styles/global.css`

Refonte complete des fondations visuelles. Utilise `@theme` de Tailwind v4 pour injecter
des tokens custom directement utilisables comme classes Tailwind (`bg-surface-base`, etc.).

### 1.1 Tokens via @theme (Tailwind v4 natif)

```css
@import "tailwindcss";

@theme {
  /* Surfaces (elevation-based, dark) */
  --color-surface-base:    #0f1117;
  --color-surface-raised:  #161922;
  --color-surface-overlay: #1c1f2b;
  /* Surfaces (light) */
  --color-surface-base-light:    #f8f9fb;
  --color-surface-raised-light:  #ffffff;
  --color-surface-overlay-light: #f1f3f7;
  /* Bordures (opacite-based, dark) */
  --color-border-subtle:   rgba(255,255,255,0.06);
  --color-border-default:  rgba(255,255,255,0.10);
  --color-border-strong:   rgba(255,255,255,0.16);
  /* Bordures (light) */
  --color-border-subtle-lt:  rgba(0,0,0,0.06);
  --color-border-default-lt: rgba(0,0,0,0.10);
  --color-border-strong-lt:  rgba(0,0,0,0.14);
  /* Accent */
  --color-accent:       #3b82f6;
  --color-accent-hover: #2563eb;
  --color-accent-muted: rgba(59,130,246,0.15);
  --color-accent-glow:  rgba(59,130,246,0.25);
  /* Shadows */
  --shadow-card:    0 1px 2px rgba(0,0,0,0.3), 0 1px 3px rgba(0,0,0,0.15);
  --shadow-card-lt: 0 1px 2px rgba(0,0,0,0.04), 0 1px 3px rgba(0,0,0,0.06);
  --shadow-float:   0 4px 8px rgba(0,0,0,0.3), 0 8px 24px rgba(0,0,0,0.2);
}
```

### 1.2 Utilitaires CSS custom

```css
/* Glow pulse animee pour dots status */
@keyframes glow-pulse {
  0%, 100% { box-shadow: 0 0 4px 1px var(--glow-color); }
  50%      { box-shadow: 0 0 10px 3px var(--glow-color); }
}
.status-glow { animation: glow-pulse 2s ease-in-out infinite; }

/* Glassmorphism (overlay pill) */
.glass {
  background: rgba(15,17,23,0.72);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border: 1px solid rgba(255,255,255,0.08);
}
/* Glassmorphism plus opaque (overlay panel) */
.glass-panel {
  background: rgba(15,17,23,0.82);
  backdrop-filter: blur(24px) saturate(180%);
  -webkit-backdrop-filter: blur(24px) saturate(180%);
  border: 1px solid rgba(255,255,255,0.06);
}

/* Animation expand overlay */
@keyframes slide-down {
  from { opacity: 0; transform: translateY(-6px) scale(0.98); }
  to   { opacity: 1; transform: translateY(0) scale(1); }
}
.animate-slide-down { animation: slide-down 200ms cubic-bezier(0.16,1,0.3,1); }

/* Micro-interaction press */
.press-scale { transition: transform 100ms ease; }
.press-scale:active { transform: scale(0.97); }
```

### 1.3 Modifications existantes dans global.css
- **Retirer** la regle `* { transition-property: background-color, border-color, color; }` (trop large,
  cause des saccades). Les transitions seront sur les elements interactifs via Tailwind `transition-colors`.
- Scrollbar thumb : `rgba(148,163,184,0.2)` (teinte slate)
- Selection : `rgba(59,130,246,0.2)` (plus subtil)
- Focus ring : `outline: 2px solid rgba(59,130,246,0.4)` (plus doux)
- Ajouter `button, a, [role="switch"] { transition: all 150ms ease; }` pour les elements interactifs

---

## 2. Nouveau composant - `src/components/ui/Section.tsx` (NOUVEAU FICHIER)

Remplace le pattern repete `border-t` + `h3` dans tous les onglets (utilise ~15 fois).

```
+-----------------------------------------------------+
| [|] TITRE                                  [Action]  |  <- barre bleue + titre uppercase
|-----------------------------------------------------|
|                                                     |
|  Contenu                                            |  <- px-4 py-3
|                                                     |
+-----------------------------------------------------+
```

- **Props** : `title: string`, `children: JSX.Element`, `action?: JSX.Element`
- Carte dark : `rounded-xl bg-surface-raised border border-border-subtle`
- Carte light : `rounded-xl bg-surface-raised-light border border-border-subtle-lt shadow-card-lt`
- Header : `flex items-center justify-between px-4 py-2.5 border-b border-border-subtle`
- Barre bleue : `w-1 h-3.5 rounded-full bg-accent` (avant le titre)
- Titre : `text-xs font-semibold uppercase tracking-wider` + opacite secondaire

---

## 3. Layout - `src/components/layout/PageShell.tsx`

### Root div
- Dark : `h-screen flex flex-col bg-surface-base`
- Light : `h-screen flex flex-col bg-surface-base-light`
- Texte : utiliser `text-white/92` (dark) / `text-black/88` (light) pour le texte primaire

### Header
- Retirer `border-b`, ajouter `shadow-card` (dark) / `shadow-card-lt` (light) pour profondeur
- "Voxai" : `text-lg font-bold tracking-tight`
- Sous-titre : `text-xs` + opacite tertiaire

### Theme toggle : remplacer emojis par SVG

**Soleil (mode dark -> basculer en light)** :
```html
<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor"
     stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <circle cx="12" cy="12" r="5"/>
  <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42
           M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
</svg>
```

**Lune (mode light -> basculer en dark)** :
```html
<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor"
     stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
</svg>
```

Bouton : `w-8 h-8 flex items-center justify-center rounded-lg transition-colors
          hover:bg-surface-overlay` (dark) / `hover:bg-surface-overlay-light` (light)

### Status bar wrapper
- Retirer `border-b` (la carte status a sa propre bordure)
- `shrink-0 px-5 py-2`

### Tab bar wrapper
- `shrink-0 px-5 pt-2 pb-1` (plus de respiration)

### Error banner
- `rounded-xl` au lieu de `rounded-lg`

---

## 4. Onglets pilules - `src/components/layout/TabBar.tsx`

Design : pilules dans un conteneur tinte arrondi.

**Conteneur** :
- Dark : `flex gap-1 rounded-xl p-1 bg-surface-raised`
- Light : `flex gap-1 rounded-xl p-1 bg-surface-overlay-light`

**Onglet actif** :
- Dark : `px-3.5 py-1.5 text-xs font-semibold rounded-lg bg-surface-overlay text-white shadow-card transition-all`
- Light : `px-3.5 py-1.5 text-xs font-semibold rounded-lg bg-white text-black/88 shadow-card-lt transition-all`

**Onglet inactif** :
- Dark : `px-3.5 py-1.5 text-xs font-medium rounded-lg text-white/44 hover:text-white/64 hover:bg-white/5 transition-all`
- Light : `px-3.5 py-1.5 text-xs font-medium rounded-lg text-black/40 hover:text-black/60 hover:bg-black/5 transition-all`

---

## 5. Status bar - `src/App.tsx`

Design : carte arrondie avec bande d'accent coloree a gauche, dot avec effet glow.

```
+--+------------------------------------------------+
|  | * Ready                [Ctrl+Shift+Space]       |
+--+------------------------------------------------+
 ^    ^                      ^
 bande dot (glow)           kbd badge
```

**Structure** :
```tsx
<div class="rounded-xl overflow-hidden flex items-stretch bg-surface-raised border border-border-subtle">
  <div class={`w-1 shrink-0 ${accentBg()}`} />
  <div class="flex items-center gap-3 px-3 py-2.5 flex-1">
    <div class={`w-2 h-2 rounded-full ${dotBg()} ${isActive ? "status-glow" : ""}`}
         style={{ "--glow-color": glowColor }} />
    <span class="text-sm font-medium">{statusText()}</span>
    <kbd class="ml-auto px-2 py-0.5 rounded-md text-xs font-mono bg-surface-overlay
                border border-border-subtle text-white/44">...</kbd>
  </div>
</div>
```

**Couleurs status** :
| Etat | Bande | Dot | Glow |
|------|-------|-----|------|
| Idle | `bg-emerald-500` | `bg-emerald-400` | `#34d399` |
| Recording | `bg-red-500` | `bg-red-400` | `#f87171` |
| Processing | `bg-amber-500` | `bg-amber-400` | `#fbbf24` |

Remplacer `animate-pulse` par `status-glow` (animation CSS custom plus raffinee).

---

## 6. Composants UI de base

### 6.1 Button.tsx
- Base : `rounded-md font-medium inline-flex items-center justify-center transition-all press-scale`
- **primary** : `bg-blue-600 hover:bg-blue-500 text-white shadow-sm shadow-blue-600/20`
- **secondary** dark : `bg-surface-overlay text-white/64 border border-border-subtle hover:bg-white/10`
- **secondary** light : `bg-surface-overlay-light text-black/60 border border-border-subtle-lt hover:bg-black/5`
- **danger** : `bg-red-600 hover:bg-red-500 text-white shadow-sm shadow-red-600/20`
- `press-scale` -> `active:scale(0.97)` via la classe CSS

### 6.2 Toggle.tsx
- Track : **22px haut x 42px large** (`h-[22px] w-[42px] rounded-full`)
  - ON : `bg-blue-500`
  - OFF dark : `bg-white/10`
  - OFF light : `bg-black/15`
- Thumb : **16px x 16px** (`h-4 w-4 rounded-full bg-white shadow-sm`)
  - ON : `translate-x-[22px]`
  - OFF : `translate-x-[3px]`
- `transition-all duration-200`
- Description : opacite tertiaire

### 6.3 Select.tsx
- Wrapper `relative` pour positionner le chevron
- `appearance-none` + `pr-9` (place pour le chevron)
- `rounded-md focus:outline-none focus:ring-2 focus:ring-accent-glow focus:border-accent`
- Dark : `bg-surface-raised border-border-default text-white/92`
- Light : `bg-white border-border-default-lt text-black/88`
- Chevron SVG (`absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none w-4 h-4`)

### 6.4 Input.tsx
- `rounded-md focus:outline-none focus:ring-2 focus:ring-accent-glow focus:border-accent transition-shadow`
- Dark : `bg-surface-raised border-border-default text-white/92 placeholder:text-white/30`
- Light : `bg-white border-border-default-lt text-black/88 placeholder:text-black/30`

### 6.5 ProgressBar.tsx
- Track : `h-2 rounded-full` dark `bg-white/8` / light `bg-black/8`
- Fill : `bg-gradient-to-r from-blue-600 to-blue-400 rounded-full transition-all duration-300`
- Label : opacite tertiaire

---

## 7. Onglets Settings (5 fichiers) - Migration

### Pattern commun
**AVANT** (repete ~15 fois) :
```tsx
<div class="border-t border-gray-800 pt-4 mt-4">
  <h3 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-2">Title</h3>
  ...
</div>
```
**APRES** :
```tsx
<Section title={title}>
  ...
</Section>
```

### 7.1 GeneralTab.tsx - 7 sections
`Input`, `Behavior`, `Mode`, `Hotkey`, `Text Hotkey`, `GPU`, `Performance`
- Root : `<div class="space-y-3">`
- Sous-cartes hotkey : `rounded-lg p-3 bg-white/5 border border-border-subtle` (dark)
- KBD : `px-2 py-0.5 rounded-md text-xs font-mono bg-surface-overlay border border-border-default text-white/64`

### 7.2 EnginesTab.tsx
- Chaque engine dans `<Section title={name} action={badge}>`
- Model cards : `rounded-lg p-3 bg-white/5 border border-border-subtle` (dark)
- Badge active : `bg-accent-muted text-blue-400 ring-1 ring-blue-500/20 rounded-full px-2 py-0.5 text-xs`
- Badge loaded : `bg-emerald-500/15 text-emerald-400 ring-1 ring-emerald-500/20 rounded-full px-2 py-0.5 text-xs`

### 7.3 PostProcessingTab.tsx - 5 sections
`Text Cleanup`, `LLM Backend` (action=Refresh), `Translation`, `Reformulation`, `Test Zone`
- Style radio list : bordure `border-border-default`, row active `bg-accent-muted`, hover `hover:bg-white/5`
- Radio dot active : `border-accent bg-accent`, inactive `border-white/20`
- Textareas : memes classes que Input (rounded-md, focus ring, etc.)

### 7.4 SubstitutionsTab.tsx
- "Add rule" dans `<Section>`
- Rules : `rounded-lg px-3 py-2 bg-white/5 border border-border-subtle`
- Code from/to : `text-red-400`/`text-emerald-400` (dark), `text-red-600`/`text-emerald-600` (light)

### 7.5 HistoryTab.tsx
- Cards dark : `rounded-lg p-3 bg-surface-raised border border-border-subtle`
- Cards light : `rounded-lg p-3 bg-white border border-border-subtle-lt shadow-card-lt`
- Metadata : opacite tertiaire

---

## 8. Overlay - `src/Overlay.tsx`

### 8.1 Pill header
- Classes : `.glass rounded-full shadow-float` (au lieu de `bg-gray-900/80 backdrop-blur-sm`)
- Glow micro : `filter: drop-shadow(0 0 6px ${statusColor()})` quand anime
- Texte : `text-xs font-medium text-white/90`
- Chevron SVG (`w-3 h-3`) avec `transition-transform duration-200` + `rotate-180` quand expanded

### 8.2 Panel expanded
- Classes : `.glass-panel .animate-slide-down rounded-xl shadow-float`
- Espacement : `p-3 space-y-3` (plus aere que l'actuel `p-2 space-y-2`)
- Labels : `text-xs font-medium text-white/50 w-20 shrink-0`

### 8.3 Mini toggles
- Track : `w-8 h-[18px] rounded-full bg-white/10 peer-checked:bg-blue-500`
- Thumb : `h-3.5 w-3.5 rounded-full bg-white shadow-sm`

### 8.4 Mini selects
- `bg-white/8 text-white/80 text-[10px] rounded-md px-1.5 py-0.5 border border-white/10
   focus:ring-1 focus:ring-accent-glow`

### 8.5 Warning
- `rounded-md px-2 py-1 bg-amber-500/10 border border-amber-500/20 text-amber-400 text-[10px]`

### 8.6 Streaming text
- `.glass-panel rounded-xl p-2.5 shadow-float text-xs text-white/85`

---

## 9. Table de migration couleurs (tous fichiers)

| Actuel | Nouveau |
|--------|---------|
| `bg-gray-900` (fond) | `bg-surface-base` |
| `bg-gray-800` (cartes) | `bg-surface-raised` |
| `bg-gray-800/50` | `bg-white/5` |
| `bg-gray-700` (hover/overlay) | `bg-surface-overlay` |
| `bg-white` (fond light) | `bg-surface-base-light` |
| `bg-gray-50` (cartes light) | `bg-surface-raised-light` |
| `bg-gray-100` (hover light) | `bg-surface-overlay-light` |
| `text-gray-100` | `text-white/92` |
| `text-gray-200/300` | `text-white/64` |
| `text-gray-400` | `text-white/64` |
| `text-gray-500` | `text-white/44` |
| `text-gray-900` (light) | `text-black/88` |
| `text-gray-600/700` (light) | `text-black/60` |
| `border-gray-700/800` | `border-border-default` |
| `border-gray-200/300` | `border-border-default-lt` |
| `hover:bg-gray-800` | `hover:bg-white/5` |
| `hover:bg-gray-100/200` | `hover:bg-black/5` |
| `bg-green-500` (status) | `bg-emerald-400` |
| `bg-yellow-500` (status) | `bg-amber-400` |
| `animate-pulse` (status) | `status-glow` |
| `bg-yellow-900/*` (warning) | `bg-amber-500/10` |
| `text-yellow-*` (warning) | `text-amber-400` |
| `placeholder-gray-*` | `placeholder:text-white/30` / `placeholder:text-black/30` |

---

## 10. Ordre d'implementation

| # | Fichier | Changement |
|---|---------|------------|
| 1 | `src/styles/global.css` | @theme tokens, utilitaires CSS, retrait transition globale |
| 2 | `src/components/ui/Section.tsx` | **NOUVEAU** composant Section |
| 3 | `src/components/ui/Button.tsx` | rounded-md, shadows, press-scale |
| 4 | `src/components/ui/Toggle.tsx` | taille augmentee, couleurs opacite |
| 5 | `src/components/ui/Select.tsx` | chevron SVG, focus ring, appearance-none |
| 6 | `src/components/ui/Input.tsx` | focus ring glow, couleurs opacite |
| 7 | `src/components/ui/ProgressBar.tsx` | gradient fill, couleurs opacite |
| 8 | `src/components/layout/PageShell.tsx` | SVG icons, shadow, tokens |
| 9 | `src/components/layout/TabBar.tsx` | pilules dans conteneur tinte |
| 10 | `src/App.tsx` | status bar (bande accent, glow, kbd) |
| 11 | `src/components/settings/GeneralTab.tsx` | 7x Section + couleurs |
| 12 | `src/components/settings/EnginesTab.tsx` | Section + badges + couleurs |
| 13 | `src/components/settings/PostProcessingTab.tsx` | 5x Section + radio list + couleurs |
| 14 | `src/components/settings/SubstitutionsTab.tsx` | Section + couleurs |
| 15 | `src/components/settings/HistoryTab.tsx` | cards + couleurs |
| 16 | `src/Overlay.tsx` | glass, glow, animation, chevron SVG |

---

## 11. Verification

1. `npx tsc --noEmit` - zero erreurs TypeScript
2. `npm run dev` - hot-reload fonctionnel
3. Tester dark mode : surfaces distinctes (base < raised < overlay)
4. Tester light mode : coherent, pas de blanc sur blanc
5. Tester les 5 onglets en dark ET light
6. Tester l'overlay : pill, expand/collapse, glow, toggles
7. Tester status bar : 3 etats (Idle, Recording, Processing) + glow
8. Tester micro-interactions : hover boutons, focus inputs, press-scale
9. Verifier qu'aucune fonctionnalite n'a change
