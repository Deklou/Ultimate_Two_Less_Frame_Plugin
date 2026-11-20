# État technique — 20 novembre 2026

Cible : Smash Ultimate 13.0.5, Title ID `01006A800016E000`, Build ID `21450C647B8C5940ED5B68218EA128FD`, Ryujinx portable 1.1.1403.

Skyline : binaire identique à la release officielle beta (SHA-256 `9959ECB13EAB6DB81A07CEE04A789B1905FD605409FC84EFEB7EC43DA7E29280`). ARCropolis : 4.1.1.

Less Delay : combat accéléré avec le NRO, vitesse redevenue normale après son retrait (test utilisateur sur Ryujinx). Ne pas le réinstaller pour tester ce projet.

Ghidra Smash (lecture seule) : import `nn::vi::GetDisplayVsyncEvent` à `0x71039C8180`, appel à `0x7103860C24` ; imports `nn::hid::GetNpadState`.

Objectif expérimental : gain mesurable d'environ 1 frame, puis éventuellement 2, **sans augmenter la cadence du gameplay**. Référence nécessaire : mesurer input → affichage et confirmer que la vitesse en combat reste normale.

Compilation locale : `cargo skyline build --release` terminée avec succès le 20 novembre 2026 sur WSL2 / Rust 1.100.0 / cargo-skyline 3.5.0 (sortie de compilation communiquée par l'utilisateur).

Déploiement : `bash ./build_deploy.sh` compile et copie uniquement `libremove_the_frame.nro` dans le Ryujinx portable, après contrôle du dossier et d'ARCropolis. Toute version précédente de ce même NRO est sauvegardée hors du dossier Skyline (`.deploy-backups/`). Script **exécuté avec succès par l'utilisateur** sous WSL2 après correction des fins de ligne CRLF en LF ; NRO installé dans la hiérarchie de Ryujinx portable. Fermer Ryujinx avant le déploiement.

Test Ryujinx du 20 novembre 2026 (log `Ryujinx_1.1.1403_2026-11-20_09-31-13.log`, fourni par l'utilisateur) : `libremove_the_frame.nro` lu, chargé, `main` exécuté et message `[remove-the-frame] Chargement de test : aucun hook actif` présent. Combat hors ligne contre CPU terminé, vitesse normale et absence de crash confirmées par l'utilisateur. Les avertissements `TerminationRequested` à l'arrêt ne démontrent pas un crash du plugin.

Statut : **compilation, déploiement et chargement du plugin minimal validés en jeu sur Ryujinx**. Aucun hook actif, **aucune réduction de latence à ce stade**. Prochaine étape : identifier et vérifier un hook de synchronisation, puis tester une seule modification à la fois.

## Sonde VSync (branche de test, 20 novembre 2026)

Dans la zone de code étudiée, le passage 13.0.4 → 13.0.5 ajoute `0x5B0` aux offsets (voir [VERSIONS.md](https://github.com/OpenSmash-Ultimate/ssbu-recomp/blob/main/docs/VERSIONS.md)). Ce décalage a été contrôlé dans Ghidra, pas supposé applicable au binaire entier.

La signature `vsync_count_thread` publiée par [Naxdy/less-delay](https://github.com/Naxdy/less-delay/blob/master/src/offsets.rs) se retrouve dans notre `main` 13.0.5 à `0x7103811014` (offset Skyline `0x3811014`) : `a8 fe 5f c8 08 05 00 91 a8 fe 09 c8...`. Elle n'est pas au même offset que dans les anciens mods (`0x3810A64`). **Concordance de signature Ghidra confirmée**, rôle et fréquence réelle du callback **à confirmer en jeu**.

Le prototype installe **un seul hook inline passif** à cette adresse après vérification des deux premiers mots AArch64. Il incrémente un compteur atomique et émet **une seule ligne de log à 120 passages**. Il ne change ni la VSync, ni les buffers, ni les registres du CPU. Aucun gain de latence n'est revendiqué. **Nouvelle version : pas encore compilée ni testée en jeu.**

Autre piste trouvée : à `0x710386B0FC` (offset `0x386B0FC`), Ghidra montre `29 09 00 11` = `add w9, w9, #2`, ce qui correspond au site de choix d'index étudié dans [ssbu-less-lag-3-frames-stutter](https://github.com/alggalin/ssbu-less-lag-3-frames-stutter/blob/console/src/swapchain.rs) après conversion d'offset. **Ne pas modifier cette instruction isolément** : une désynchronisation des textures / files de rendu est possible. Valider d'abord la sonde et le pipeline.
