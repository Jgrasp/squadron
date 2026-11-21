# Squadron

Avant toute maquette, artifact, bloc ```html ou retouche de style : lire `DESIGN.md` et le `:root` de `src/app.css`, n'utiliser que ces tokens et classes, partir du CSS du composant existant le plus proche.

Un commit qui ajoute ou change un token, une couleur en dur, une taille ou un composant visible met `DESIGN.md` à jour dans le même commit. `scripts/design.test.ts` (dans `pnpm test`) échoue si un token de `app.css` ou une couleur en dur des composants n'y figure pas.

Tout passe par une PR vers `main`, fusionnée par merge commit une fois verts ses checks obligatoires `test` (build, svelte-check, vitest, `cargo fmt --check`, clippy, `cargo test`) et `secrets` (gitleaks, règles dans `.gitleaks.toml`) : pas besoin de les rejouer avant de committer. Chaque push sur `main` rejoue `test` et `build` pour garder le cache Rust ; seule la poussée d'un tag `v*` publie une version : le workflow `Tag` le pose à la fusion d'une PR `release/v*` avec le secret `RELEASE_TAG_TOKEN` (token de Quentin, la règle « release tags » réservant les tags `v*` à l'admin), `scripts/release.sh tag` en secours. Un commit qui change les jobs `release`, `build` ou `publish`, ou `.actrc` et `.github/act/`, passe d'abord `pnpm act` (pipeline Linux d'un tag en local via Docker, environ 7 min) jusqu'au bout sans échec.

Recette autonome : `pnpm app --headless` lance l'app de dev du worktree sous Xvfb, invisible ; `node scripts/drive.mjs run <script.js|->` exécute un script dans la fenêtre principale et affiche sa valeur de retour, `node scripts/drive.mjs shot` écrit une capture PNG de chaque fenêtre et en donne les chemins. Les projets sont ceux de la vraie app : ne rien y modifier pendant une recette.
