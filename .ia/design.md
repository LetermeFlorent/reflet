# Reflet — Design system (formalisation)

> Ce document **formalise le design DEJA implemente**, lu directement dans
> `src/styles/tokens.css` et `src/styles/global.css`.
> Statut global : **valide de facto, a confirmer** par l'utilisateur.
> Tout ce qui doit etre confirme ou clarifie est marque **[A CONFIRMER]**.

Reflet est une app desktop **100% locale** (Tauri 2 + SvelteKit). Les textes
sont en francais code en dur (pas d'i18n). Aucune dependance design externe :
les tokens sont des variables CSS natives.

---

## 1. Principe directeur : natif Windows / Fluent

Le design vise un rendu **natif Windows (Fluent System)**, sans habillage de marque :

- **Polices systeme** uniquement (pas de webfont chargee).
- **Couleur d'accent = couleur d'accent systeme** (mots-cles CSS `AccentColor`,
  `AccentColorText`), donc l'app suit le theme choisi par l'utilisateur dans Windows.
- **Theme clair + sombre automatique** via `prefers-color-scheme` (pas de bascule
  manuelle dans l'UI). **[A CONFIRMER]** : pas de toggle theme prevu cote app.
- Surfaces sobres : pas d'ombre sur les cartes en clair, ombres reservees aux modales.

---

## 2. Typographie

Source : `tokens.css` (`--font`) et `global.css`.

- **Pile de polices** (token `--font`) :
  `-apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, Inter, Roboto,
  "Helvetica Neue", Arial, sans-serif`
  → priorite a la police systeme (Segoe UI sur Windows). `Inter`/`Roboto` ne sont
  que des **secours non charges**. **[A CONFIRMER]** : confirmer qu'on reste 100% systeme
  (aucun webfont embarque).
- **Police monospace** (classe `.mono`, non tokenisee) :
  `ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`, 12px — utilisee pour
  les chemins / valeurs techniques.
- **Texte courant** : 14px, `line-height` 1.45, lissage `antialiased`.
- **Titres** : `font-weight: 600`, `letter-spacing: -0.01em`.
  - `h1` = 22px, `h2` = 16px. `h3` partage les styles de titre **[A CONFIRMER]**
    (declare dans le selecteur groupe mais sans taille propre → herite).
- **Labels de champ** (`.label`) : 12px, gras, couleur secondaire (`--text-2`).

---

## 3. Espacement — grille 4px

Echelle d'espacement basee sur un pas de **4px** (tokens `--s1`..`--s6`) :

| Token  | Valeur |
|--------|--------|
| `--s1` | 4px    |
| `--s2` | 8px    |
| `--s3` | 12px   |
| `--s4` | 16px   |
| `--s5` | 24px   |
| `--s6` | 32px   |

Usages observes : padding de page `var(--s5) var(--s6)`, padding modale `var(--s5)`,
gap de ligne (`.row`) `var(--s3)`, padding backdrop `var(--s4)`.
**[A CONFIRMER]** : certaines valeurs restent en pixels « durs » hors echelle
(hauteurs de controles 22/25/28px, gaps 6px, paddings boutons 12/16px) — a confirmer
si on veut les ramener strictement sur la grille 4px.

---

## 4. Rayons (border-radius)

Trois rayons semantiques selon le niveau de surface :

| Token        | Valeur | Usage           |
|--------------|--------|-----------------|
| `--r-control`| 6px    | boutons, champs |
| `--r-card`   | 10px   | cartes          |
| `--r-modal`  | 14px   | modales         |

(Cas particuliers non tokenises : pastille `.dot` et thumb du switch en `50%`,
rail du switch `24px`, scrollbar `8px`.)

---

## 5. Palette de couleurs (clair + sombre)

Pilotee par `prefers-color-scheme`. Les couleurs d'accent ne sont **pas** dans la
palette : elles viennent du systeme (section 6).

### Surfaces & texte

| Token            | Clair                      | Sombre                       | Role                              |
|------------------|----------------------------|------------------------------|-----------------------------------|
| `--bg`           | `#f3f3f3`                  | `#202020`                    | fond fenetre                      |
| `--bg-elev`      | `#fbfbfb`                  | `#2b2b2b`                    | surface elevee (cartes, champs)   |
| `--bg-sunken`    | `#eaeaea`                  | `#1a1a1a`                    | surface en creux                  |
| `--text`         | `#1a1a1a`                  | `#f2f2f2`                    | texte principal                   |
| `--text-2`       | `#5e5e5e`                  | `#a0a0a0`                    | texte secondaire / muted          |
| `--border`       | `rgba(0,0,0,0.12)`         | `rgba(255,255,255,0.14)`     | bordures de controles             |
| `--hairline`     | `rgba(0,0,0,0.09)`         | `rgba(255,255,255,0.09)`     | bordures fines (cartes)           |
| `--hover`        | `rgba(0,0,0,0.05)`         | `rgba(255,255,255,0.06)`     | survol                            |
| `--shadow-card`  | `none`                     | `none`                       | cartes sans ombre                 |
| `--shadow-modal` | `0 8px 30px rgba(0,0,0,.18)`| `0 10px 40px rgba(0,0,0,.6)`| ombre des modales                 |

### Couleurs de statut

| Token     | Clair       | Sombre      | Sens (de facto)                     |
|-----------|-------------|-------------|-------------------------------------|
| `--green` | `#2eae4e`   | `#30d158`   | succes / actif (switch ON)          |
| `--red`   | `#e5484d`   | `#ff5a5f`   | erreur / danger / suppression       |
| `--orange`| `#f5a623`   | `#ff9f0a`   | avertissement                       |
| `--gray`  | `#8a8a8a`   | `#8a8a8a`   | neutre / inactif (seul ton identique clair/sombre) |

**[A CONFIRMER]** : la semantique exacte de chaque couleur de statut (notamment
orange = avertissement, gris = inactif) est deduite de l'usage, a valider.

---

## 6. Couleur d'accent systeme

Tokens dedies, distincts de la palette (`tokens.css`) :

| Token           | Valeur          | Note                                                        |
|-----------------|-----------------|-------------------------------------------------------------|
| `--accent`      | `AccentColor`     | couleur d'accent Windows                                  |
| `--accent-press`| `AccentColor`     | identique a `--accent` (l'effet pressed passe par `filter: brightness(1.12)`) |
| `--accent-text` | `AccentColorText` | couleur de texte lisible sur l'accent                     |

Usages : bouton primaire, focus des champs (bordure + halo
`box-shadow 0 0 0 3px` accent a 25%).
**[A CONFIRMER]** : `--accent` et `--accent-press` sont aujourd'hui identiques ;
confirmer si un vrai ton « pressed » distinct est souhaite ou si le `brightness`
suffit.

---

## 7. Boutons

> **Principe demande** : « boutons uniformes (plats, pas d'accent) ».
> **Etat reel du code** : le bouton **par defaut** (`.btn`) est bien **plat et
> neutre** (fond `--bg-elev`, bordure `--border`, texte `--text`, **sans accent**).
> Il existe toutefois des **variantes** dans `global.css`. **[A CONFIRMER]** :
> aligner principe et code (soit le principe « pas d'accent » vaut pour la base et
> tolere une variante primaire, soit supprimer/limiter `.btn-primary`).

Specs de la base `.btn` :

- hauteur **25px**, padding `0 16px`, rayon `--r-control` (6px),
- bordure `1px solid var(--border)`, fond `var(--bg-elev)`, texte `var(--text)`,
- survol : fond `var(--hover)` ; appui : `transform: scale(0.97)` ;
  desactive : `opacity 0.45`,
- transitions sur `background`, `transform`, `border-color` avec `--ease`.

Variantes (modificateurs) :

| Classe        | Effet                                                                 |
|---------------|-----------------------------------------------------------------------|
| `.btn-primary`| fond = `--accent`, bordure transparente, texte `--accent-text` (**utilise l'accent**) |
| `.btn-danger` | texte `--red`, survol fond rouge a 12%                                |
| `.btn-ghost`  | fond et bordure transparents (totalement plat)                        |
| `.btn-sm`     | hauteur 22px, padding `0 12px`, 13px                                  |
| `.btn-icon`   | largeur 28px, sans padding (bouton icone)                             |

---

## 8. Autres composants tokenises

- **Carte** (`.card`) : fond `--bg-elev`, bordure `--hairline`, rayon `--r-card`,
  ombre `--shadow-card` (none), transitions ombre/transform.
- **Champ** (`.input`, `.select`) : hauteur 22px, rayon `--r-control`, bordure
  `--border` ; focus = bordure accent + halo accent 25%. `textarea.input` :
  hauteur auto, padding `8px 10px`, resize vertical.
- **Switch** (`.switch`) : rail 34x20px, thumb 16px blanc avec ombre legere ;
  OFF = `--gray`, ON = `--green` (thumb translate de 14px).
- **Badge / pastille** (`.badge`, `.dot`) : 12px gras `--text-2`, pastille 8px
  ronde (defaut `--gray`).
- **Modale** (`.modal` + `.modal-backdrop`) : backdrop noir 32%, modale largeur
  max 560px, rayon `--r-modal`, ombre `--shadow-modal`, padding `--s5`.

---

## 9. Mouvement (animations)

- **Courbe d'easing unique** : token `--ease` = `cubic-bezier(0.2, 0.8, 0.2, 1)`.
- Keyframes : `fadeIn` (backdrop), `modalPop` (modale), `pageIn` (entree de page).
- Durees courtes (0.1s a 0.25s).
- **Accessibilite** : `prefers-reduced-motion: reduce` neutralise animations et
  transitions (passees a `0.001ms`). Bon point a conserver.

---

## 10. Divertissements / details systeme

- `overflow: hidden` global + `overscroll-behavior: none` (fenetre desktop, pas
  de rebond de scroll). Conteneurs scrollables : `overscroll-behavior: contain`.
- **Scrollbar** stylee (WebKit) : 10px, thumb `--border` arrondi.
  **[A CONFIRMER]** : style scrollbar uniquement WebKit/Chromium (OK pour la
  webview Tauri sous Windows ; a noter si support d'autres moteurs un jour).

---

## Points a confirmer (recapitulatif)

1. **Boutons** : principe « pas d'accent » vs existence de `.btn-primary` (accent). (section 7)
2. **Accent pressed** : `--accent` == `--accent-press`, garder ou differencier. (section 6)
3. **Polices** : confirmer 100% systeme, aucun webfont embarque. (section 2)
4. **`h3`** : taille volontairement heritee ou a definir. (section 2)
5. **Grille 4px** : tolerer les valeurs px hors echelle (hauteurs/gaps) ou normaliser. (section 3)
6. **Semantique statuts** : valider sens vert/rouge/orange/gris. (section 5)
7. **Theme** : confirmer l'absence de toggle manuel (auto `prefers-color-scheme`). (section 1)
8. **Scrollbar** : style WebKit-only assume. (section 10)
