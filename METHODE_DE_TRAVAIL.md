# Méthode de travail pour les prochains mods

Mise à jour du 20 novembre 2026, à partir du retour d'expérience sur le plugin de latence Smash Ultimate. Règles réutilisables pour les projets de modding ; les détails techniques propres à Smash sont dans `ETAT_TECHNIQUE.md`.

## 1. Décider de la viabilité avant de développer

Avant le premier hook, fixer en quelques lignes : comportement voulu, bénéfice attendu, plateforme/version, solution connue ou seulement supposée, preuve minimale de réussite, budget de temps et critère d'arrêt. Si le bénéfice est petit et la vérification coûteuse, faire d'abord une courte étude de faisabilité et décider de poursuivre ou non. Ne pas promettre un résultat simplement parce qu'un autre plugin fonctionne sur une autre plateforme.

**Règle :** ne pas multiplier les essais si aucun critère objectif ne peut départager deux versions. Un ressenti peut orienter la recherche ; il ne prouve pas un gain d'une frame.

## 2. Une branche expérimentale active, une PR brouillon

- `main` reste stable. Pour une série d'essais sur le même objectif, utiliser **une seule branche de test et une seule PR brouillon**, avec un commit court et descriptif par hypothèse.
- Ne créer une autre branche que pour un travail réellement parallèle ou une alternative nécessitant une isolation ; expliquer pourquoi avant de la créer.
- Fermer les essais abandonnés sans fusion. Ne pas empiler les bases de PR ni demander de supprimer les branches une par une.
- Avant une modification : vérifier le dépôt, la branche, le commit, les fichiers locaux non suivis (notamment `Cargo.lock`) et l'installation cible.
- Ne jamais fusionner, réécrire l'historique, supprimer des branches locales ou modifier les sauvegardes/configurations de l'émulateur sans accord. Conserver des commits et références de récupération.

## 3. Vérifier les faits avant de coder

Lire les sources existantes, la version exacte du jeu et les dépendances ; inspecter signatures, fonctions, ABI, offsets et appelants dans Ghidra **en lecture seule** quand c'est nécessaire. Un résultat obtenu sur Switch physique ne valide pas le même comportement sous Ryujinx. Séparer :
- **Code confirmé** : instruction, symbole, xref ou signature vérifiée dans le bon binaire.
- **Compilation / chargement** : NRO construit puis chargé selon sorties de commande et log.
- **Test en jeu** : comportement observé dans un scénario connu.
- **Mesure de performance** : protocole comparatif suffisamment précis pour l'effet recherché.
- **Hypothèse / inconnu** : ne pas présenter comme un fait acquis.

## 4. Cycle de test minimal

1. Définir une hypothèse et **un seul critère de réussite ou d'échec** avant de modifier le code.
2. Changer le minimum de code ; ne pas ajouter de sonde ou de dépendance qui ne répond pas directement à la question.
3. Compiler et vérifier le binaire réellement déployé, en préservant les installations, mods et sauvegardes.
4. Tester une situation reproductible et fournir le log utile ; un crash appelle une **analyse des logs et un retour au point stable**, pas trois correctifs successifs au hasard.
5. Consigner « confirmé / non confirmé / échec / reste inconnu » puis **décider de continuer, changer de piste ou arrêter**. N'ajouter ni branche ni PR à chaque étape.

Pour une optimisation de latence, fixer la méthode de mesure **avant** les patches. Les compteurs CPU (input, logique, `Present`) ne donnent pas à eux seuls l'instant de visibilité à l'écran. Une comparaison A/B fiable doit respecter la précision visée, les conditions identiques et les effets indésirables (cadence, saccades, crashs).

## 5. Documentation courte, orientée reprise

- **README** : une phrase expliquant l'objet du plugin.
- **Note d'état** : objectif, environnement, référence stable, branche active, résultat validé, principal blocage et prochaine décision utile.
- **Journal technique** : uniquement les fonctions/offsets/signatures/Build IDs, tests déterminants, logs et références de commits nécessaires à reproduire les conclusions.
- Éviter les répétitions, les comptes rendus d'étapes triviales et les promesses de résultats. Mettre les conclusions actuelles en tête ; conserver l'historique en dessous.
- À la fin de la séance, vérifier que le projet est **reprenable sans reposer les mêmes questions**.

## 6. Retour d'expérience : plugin de latence Smash

Sur Ryujinx 1.1.1403 / Smash 13.0.5, le plugin de diagnostic a suivi des appuis candidats (`0x20`) jusqu'à `JumpSquat` puis `Jump`. Sur le dernier essai, 20 entrées de chacun de ces deux états ont été observées, séparées de trois indices `Present` ; cela n'établit aucun gain d'input lag. Les patches de rendu testés n'ont apporté aucun gain prouvé ; une tentative de dispatch a provoqué un écran noir. **Aucune réduction d'une ou deux frames n'est démontrée sur Ryujinx, et il n'existe pas de méthode actuellement garantie pour l'obtenir.**

Leçon : une étude de faisabilité et un critère d'arrêt auraient dû précéder l'accumulation des sondes, des branches et des essais GPU. Le projet peut être mis en pause sans perdre ses découvertes.
