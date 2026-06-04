J'ai une app desktop Tauri v2 + SvelteKit 5 (Svelte 5) sur Windows.

**Le bug :** Au démarrage de l'app, un écran de chargement (loader) s'affiche pendant au moins 3 secondes. Il montre :
- Une icône ◑ (OK)
- Une barre de progression horizontale (200px × 6px, arrondie)
- Un texte en pourcentage (ex: "0%", "1%", …, "100%")

Le **pourcentage** se met à jour correctement (0 → 100 en 3s). Mais la **barre de progression reste visuellement vide** (comme si elle faisait 0% de large) pendant tout le chargement, même si le DOM a bien la bonne largeur.

**Ce qui a déjà été testé sans succès :**
1. `<span>` avec `style="width: {pct}%"` en Svelte réactif (`$state`)
2. `<div>` avec `bind:this` + `element.style.width` en JS direct
3. `background-size` avec `linear-gradient` changeant via JS
4. `background-size` avec `@keyframes` animation CSS pure (pas de JS)
5. La couleur de remplissage s'affiche correctement (noir en mode clair, blanc en mode sombre) — c'est la largeur qui ne se rend pas

**Ce qui est étrange :** Même l'animation CSS pure (`@keyframes` qui anime `background-size` de 0% à 100%) ne fonctionne pas visuellement. Le texte `textContent` mis à jour via `requestAnimationFrame` fonctionne parfaitement.

Le loader est affiché dans un layout Svelte avec `{#if !store.loaded || !minElapsed}` (au moins 3s). Il utilise les CSS custom properties globales (`--bg`, `--hover`, `--accent`, etc.) définies dans `:root`.

Le contexte : c'est une app Tauri v2, donc WebView2 (Edge Chromium). Le problème semble lié au fait que le loader s'affiche très tôt dans le bootstrap — peut-être avant que le pipeline de rendu ne soit complètement initialisé.

**Question :** Quelle est la cause racine et comment la résoudre ? La solution doit permettre à la barre de remplir visuellement de 0% à 100% sur 3 secondes.
