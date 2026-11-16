# Squadron

Avant toute maquette, artifact, bloc ```html ou retouche de style : lire `DESIGN.md` et le `:root` de `src/app.css`, n'utiliser que ces tokens et classes, partir du CSS du composant existant le plus proche.

Un commit qui ajoute ou change un token, une couleur en dur, une taille ou un composant visible met `DESIGN.md` à jour dans le même commit. `scripts/design.test.ts` (dans `pnpm test`) échoue si un token de `app.css` ou une couleur en dur des composants n'y figure pas.

Le job `test` du workflow Release (build, svelte-check, vitest, `cargo fmt --check`, clippy, `cargo test`) tourne sur chaque PR vers `main`, avant la fusion : pas besoin de le rejouer avant de committer. Un commit qui change les jobs `release`, `build` ou `publish`, qui ne tournent qu'après la fusion, ou `.actrc` et `.github/act/`, passe d'abord `pnpm act` (pipeline Linux en local via Docker, environ 7 min) jusqu'au bout sans échec.
