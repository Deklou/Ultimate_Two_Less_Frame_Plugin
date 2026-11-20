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

La signature `vsync_count_thread` publiée par [Naxdy/less-delay](https://github.com/Naxdy/less-delay/blob/master/src/offsets.rs) se retrouve dans notre `main` 13.0.5 à `0x7103811014` (offset Skyline `0x3811014`) : `a8 fe 5f c8 08 05 00 91 a8 fe 09 c8...`. Elle n'est pas au même offset que dans les anciens mods (`0x3810A64`). **Concordance de signature Ghidra confirmée**. L'exécution du hook sur Ryujinx est confirmée par le test ci-dessous ; sa fréquence exacte ne peut pas être déduite de ce seul log.

Le prototype installe **un seul hook inline passif** à cette adresse après vérification des deux premiers mots AArch64. Il incrémente un compteur atomique et émet **une seule ligne de log à 120 passages**. Il ne change ni la VSync, ni les buffers, ni les registres du CPU. Aucun gain de latence n'est revendiqué. **Cette sonde a depuis été compilée et exécutée en jeu sur Ryujinx (voir ci-dessous).**

Autre piste trouvée : à `0x710386B0FC` (offset `0x386B0FC`), Ghidra montre `29 09 00 11` = `add w9, w9, #2`, ce qui correspond au site de choix d'index étudié dans [ssbu-less-lag-3-frames-stutter](https://github.com/alggalin/ssbu-less-lag-3-frames-stutter/blob/console/src/swapchain.rs) après conversion d'offset. **Ne pas modifier cette instruction isolément** : une désynchronisation des textures / files de rendu est possible. Valider d'abord la sonde et le pipeline.

## Test de la sonde VSync — 20 novembre 2026, 09:50

Log fourni : `Ryujinx_1.1.1403_2026-11-20_09-50-58.log` (Ryujinx 1.1.1403, Smash Ultimate 13.0.5). Skyline lit et charge `libremove_the_frame.nro` ; le log indique `inline hook 0xbd15014->0x84dc21c successfully!` à 00:06.985, puis `[remove-the-frame] Sonde VSync installee (timing inchange)`. À 00:18.072 : `[remove-the-frame] VSync : 120 passages observes`. Cela **valide l'installation et l'exécution de la sonde** ; 120 passages observés ne constituent ni une mesure à 60 Hz, ni un gain de latence. Un rapport `ev_match_data` apparaît à 01:13.354, avec `rom_version` 13.0.5 et un combat de 17 secondes. Aucun crash imputable au plugin n'est identifié dans ce log ; des avertissements GPU `Votevtg`, `GPU processing thread is too slow`, et `DequeueBuffer: Busy` à l'arrêt sont présents. Ne pas attribuer ces avertissements au hook sans comparaison A/B. **Aucune baisse de latence démontrée**.

## Sonde passive d'index de rendu — préparation du prochain test

Analyse Ghidra Smash 13.0.5 **en lecture seule** : à `0x710386B0F8`, `ldr w9, [x8,#0x30]` ; à `0x710386B0FC` (offset Skyline `0x386B0FC`), `add w9,w9,#2` (`29 09 00 11`). Les instructions suivantes `6a 55 95 52 4a 55 b5 72 2a 7d aa 9b ...` calculent le choix d'index par arithmétique de modulo 3. **Ce mécanisme de sélection de texture est confirmé par les instructions**, mais pas encore son comportement effectif sur Ryujinx.

Une sonde inline lit le registre `w9` juste avant cette addition. Après 120 passages, elle journalise la répartition `w9 % 3` et la valeur maximale brute (pour distinguer un index cyclique d'un compteur non borné). Les quatre instructions initiales sont vérifiées avant de poser le hook ; en cas de différence, il reste inactif. La sonde ne touche pas aux registres, aux buffers, à la VSync ou aux cadences de jeu. **Code ajouté au dépôt, non encore compilé ni testé en jeu.**

Raison du test : la réduction de la profondeur du pipeline exige un accord entre l'index de texture, le nombre de buffers et la soumission des travaux de rendu. Modifier seulement `add ... #2` serait une hypothèse non validée pouvant afficher le mauvais buffer. Les méthodes d'autres plugins incluent plusieurs changements coordonnés et leur compatibilité avec Ryujinx 1.1.1403 n'est pas prouvée. On conserve la configuration actuelle (VSync active) et on examine d'abord la distribution d'index. 
