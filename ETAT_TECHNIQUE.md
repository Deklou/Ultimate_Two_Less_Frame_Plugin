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

## Log de l'index et mesure de phase — 20 novembre 2026

Le log `Ryujinx_1.1.1403_2026-11-20_10-04-41.log` confirme le chargement de la sonde index de rendu à `0x386B0FC` et la ligne `Rendu : 120 index, mod3=[40, 40, 40], max_brut=2` (00:21.233), avec sonde VSync à 120 passages (00:17.958). Un rapport de combat 13.0.5 de 12 s suit à 01:07.631. Ces données valident la rotation cyclique `0,1,2` du registre `w9` avant son décalage de `+2`. Elles **ne mesurent pas l'input lag**. La perception subjective d'une amélioration n'est donc pas validée par instrumentation : aucun timing ni index n'a été modifié.

Préparation de la PR suivante : Ghidra Smash 13.0.5 montre dans la boucle principale `0x7103748128: bl 0x71038701e0` (offset `0x3748128`, mots `0x9404A02E, 0xB000DF88`). La fonction cible lit le pointeur d'état de présentation (argument `x1`), fait tourner des entiers à `+0xf0..+0xfc` et soumet les opérations de présentation. Une **sonde passive supplémentaire**, avec garde de signature, lit uniquement le champ `x1+0xf0` au point d'appel pendant 120 passages. Elle journalise le cycle des valeurs `0,1,2` et la différence modulo 3 avec le dernier index de rendu observé. Le dernier index et le champ `+0xf0` ne sont **pas garantis d'appartenir à la même image** ; leur différence est un indicateur exploratoire de phase et **pas** une mesure en frames de la latence. Aucun registre, buffer, timing ou instruction métier n'est modifié par cette sonde.

**Statut de cette PR : code non compilé et non testé sur Ryujinx.** Prochaine étape : un seul test du hook et du log `Present : 120 appels`, puis décision d'un patch coordonné (soumission de commandes + index de texture), pas de changement isolé de `+2` en `+1`. La PR de base et le NRO validé sont conservés.

## Validation phase de présentation — 20 novembre 2026, 10:16

Le log Ryujinx `Ryujinx_1.1.1403_2026-11-20_10-16-37.log` confirme les 3 sondes passives : VSync (00:18.235, 120 passages), rendu et présentation (00:21.455). Observations : `slot_f0=[40, 40, 40]`, `delta=[39, 40, 40]`, `sans_correspondance=1`, `invalides=0`, rendu `mod3=[40, 40, 40], max_brut=2`. **Le delta cyclique n'établit pas un décalage stable entre images** : les deux mesures ne portent pas sur un identifiant unique de frame. Il serait incorrect d'y lire 1 ou 2 frames de retard. Aucun crash imputable au plugin ; `DequeueBuffer: Busy` apparaît à l'arrêt. Aucune baisse mesurée d'input lag.

## Contrôle de la séquence des appels — PR de test

Ghidra (lecture seule), Smash 13.0.5 : `FUN_7103725030` à `0x7103725030`, offset Skyline `0x3725030`, débute par `sub sp,sp,#0x90` puis `stp x28,x27,[sp,#0x30]` (mots `D10243FF A9036FFC`). Le site est appelé par le code de boucle principale aux adresses `0x710374C88C`, `0x710374C9C0`, `0x710374CBD4` et `0x710374CCFC`. Les éléments du code ressemblent à un gestionnaire de scènes, **sans preuve qu'un appel de cette fonction corresponde à une frame d'animation**.

Sonde inline passive : compte les appels du gestionnaire de scènes, du site d'index rendu et de la présentation. Sur 120 présentations, journalise combien d'appels rendu et scènes ont été observés entre présentations successives (0, 1, 2+). Cette statistique ne mesure pas directement la latence ni le GPU, mais aide à vérifier la séquence logique. Tous les hooks restent sans mutation du contexte CPU, du timing ou des buffers. **PR #3 : code non compilé, non testé sur Ryujinx.**

Recherche de solutions externes : certains profils coordonnent profondeur des buffers, choix d'index, synchronisation et optimisation des tâches de rendu. Ex. `Trav55555/ssbu-online-deluxe-fix` teste l'émulateur Eden, précise que les autres émulateurs ne sont pas testés, et propose des profils qui désactivent la VSync ; **ne pas transposer à Ryujinx**, où Less Delay avait accéléré le gameplay. Le patch d'index seul est exclu.

## Échec du premier patch actif — 20 novembre 2026, 10:49

La PR #4 (`test/avance-dispatch`) est **fermée sans fusion** à la suite d'un écran noir au démarrage sur Ryujinx 1.1.1403, Smash 13.0.5. Capture utilisateur : 0 FPS et aucune image. Log `Ryujinx_1.1.1403_2026-11-20_10-49-36.log` : la sonde passive se charge et le hook actif sur `0x384FA10` s'installe à 00:07.673 ; à 00:19.902, `Dispatch : index hors plage, anticipation ignoree` ; à 00:20.003, `Test dispatch actif : premiere anticipation`. Le jeu ne poursuit plus visiblement son démarrage. Pas d'exception fatale explicite dans ce log ; l'hypothèse d'un interblocage de file GPU reste à confirmer.

Le patch écrit `arg1+0x1D28` et `arg1+0x1D30` avant l'appel original. La source `alggalin/ssbu-less-lag-3-frames-stutter` (branche console, `src/swapchain.rs`) utilise une logique analogue mais décrit **la nécessité d'une coordination** avec la présentation et le flush des buffers, ainsi qu'avec l'index rendu. Notre tentative isolée a donc été imprudente : **ne pas la réactiver ni la présenter comme gain potentiel validé**. Le code du binaire à `0x710384FA10` possède un prologue cohérent avec un point d'entrée, mais Ghidra Smash ne fournit pas de fonction décompilée identifiée à cette adresse ; signature et comportement complets non confirmés. Le décalage d'offset entre versions ne suffit pas à garantir la compatibilité.

**Dernière référence testée et fonctionnelle :** branche `test/ordre-pipeline` (PR #3). Arrêter Ryujinx, revenir sur cette branche et compiler/déployer avec `bash ./build_deploy.sh` (ne change que notre NRO). Ne toucher ni aux autres mods ni aux configurations/sauvegardes. Prochain changement actif : analyser les dépendances de synchronisation, confirmer le graphe de séquence et valider un plan de rollback avant test. Éviter toute nouvelle modification d'une file seule.


## Diagnostic passif des inputs — branche de test `test/sondes-inputs-passives` (préparation)

Analyse Ghidra Smash 13.0.5 **en lecture seule** (base `0x7100000000`) :
- `0x7103667210` : routine interne de contrôle qui appelle plusieurs surcharges de `nn::hid::GetNpadState` à `0x7103667280`, `0x71036672FC`, `0x71036673A8`, `0x7103667400`, `0x7103667458`. Les appels `BL` sont confirmés par décodage des instructions ; Ghidra n'a pas défini cette routine comme fonction. Hook candidat à l'entrée, offset Skyline `0x3667210`, signature `D10183FF A9044FF4`.
- `0x7103667C90` : gestionnaire qui rafraîchit les contrôleurs via des appels virtuels ; site appelé depuis la boucle principale à `0x710374C894` et `0x710374CAB8`. Offset `0x3667C90`, signature `FC1A0FE8 A90167FA`.
- `0x7103762EC0` : copie/normalisation de contrôleurs depuis les objets globaux `0x7105339860..0x71053398A8`, appelée à `0x710375F054` ; offset `0x3762EC0`, signature `D10203FF F90013FB`.
- `0x710337B690` : traitement de masques de boutons depuis ces contrôleurs, appelée à `0x7101EF7504` dans une fonction Ghidra non définie ; offset `0x337B690`, signature `D10283FF FD001BE8`. **Le rôle précis dans la simulation du combattant n'est pas prouvé.**
- La présentation reste observée à `0x3748128` (PR #3), sans modification de l'index des textures ni du dispatch.

Code ajouté sur branche indépendante issue de PR #3 (aucun patch actif) : quatre hooks inline passifs, comptage atomique des appels, relevé des compteurs toutes les 120 présentations sur 32 fenêtres (maximum 3 840 présentations), et histogramme des appels de mappage par présentation (0/1/2+). Les sondes sont installées individuellement seulement si les deux instructions de signature correspondent. Une absence de passage est distincte d'un échec de signature par les messages d'installation. Aucun log dans les hooks de lecture de manette ; agrégation seulement au point de présentation.

**Limitations :** compter des appels HID, des copies et des présentations ne mesure pas le délai d'une pression jusqu'à sa simulation ni son affichage. Différents threads et les compteurs atomiques `Relaxed` limitent la précision des frontières temporelles ; une fenêtre de 120 présentations n'est pas nécessairement 120 frames simulées. Les données servent uniquement à confirmer la fréquence et l'ordre approximatif avant d'instrumenter une transition de bouton spécifique et un identifiant de frame corrélé. La sonde peut introduire un faible surcoût et aucun effet sur Ryujinx n'est encore validé.

**Statut : code préparé, non compilé et non testé en jeu.** Ne pas fusionner. Pour le test, fermer Ryujinx avant un déploiement, sauvegarder le NRO actuel et utiliser le script existant de build/déploiement. Test court hors ligne, vérifier l'absence de crash, le rythme normal des combats et les messages `Input diag fenetre` ; sauvegardes et mods tiers inchangés.


## Résultat en jeu PR #9 — 20 novembre 2026, log 13-44-59

Ryujinx portable 1.1.1403 / Smash 13.0.5, fichier utilisateur `Ryujinx_1.1.1403_2026-11-20_13-44-59.log` : les quatre sondes passives s'installent (lignes `lecture HID observee`, `actualisation observee`, `copie des controles observee`, `mappage observe`). 32 fenêtres de 120 présentations ont été collectées et l'arrêt normal se produit après la collecte. Pour 120 présentations, la lecture HID se produit environ 960 fois (952 au démarrage), la mise à jour 120 fois et la copie 120 fois. Ce sont des **appels observés**, pas des pressions ni des frames de latence.

Découverte : `0x337B690` est appelé lors de deux fenêtres seulement (`mappage=30` et `95` fenêtres 9 et 10), **puis zéro pendant les autres fenêtres**, y compris la phase où l'utilisateur jouait. **Cette sonde n'est donc pas retenue comme repère fiable de la commande d'un combattant en entraînement**. Les autres hooks ont tourné sans crash apparent ; la fluidité et l'absence de perturbation restent à contrôler visuellement à chaque variante.

## PR expérimentale : transitions de boutons — `test/transitions-boutons`

Branche dérivée de PR #9, sans aucun patch actif de suppression d'images ou de désactivation VSync. Le code de `src/input_trace.rs` conserve les trois sondes existantes : lecture HID `0x3667210`, rafraîchissement `0x3667C90`, copie `0x3762EC0`. Il retire la sonde `0x337B690` peu pertinente en entraînement.

Ghidra Smash (lecture seule), preuves directes de `FUN_7103762EC0` : à `0x7103762F90` la fonction lit un pointeur depuis l'array global `0x7105339860` ; `0x7103762FA0` récupère l'objet à `wrapper+0x10`, `0x7103762FA4` teste `state+0xB8` et `0x7103762FB4` lit les mots `state+0x08` et `state+0x0C`. Le premier mot représente le masque de boutons transmis à la normalisation (observé par le code, **non encore vérifié au saut en jeu**). Le nouveau hook de copie lit passivement `state+0x08` depuis le premier slot et enregistre `pressed = held & !previous` et `released = previous & !held` ; la première lecture après connexion fixe uniquement une référence, afin de ne pas générer de faux appuis. Il ne modifie jamais ce masque. Le premier slot est présumé correspondre au joueur 1 utilisé en entraînement ; à confirmer par le comportement en jeu.

Une nouvelle sonde candidate est posée à `0x71020876C0` (offset `0x20876C0`, signature `F9402000 F9400008`). Il s'agit du wrapper `app::lua_bind::StatusModule__change_status_request_impl` ; Ghidra confirme `x1` pour le code de statut demandé, à `0x71020876CC` il normalise `w2` booléen et à `0x71020876D0` il saute vers l'implémentation virtuelle. **Une demande de statut n'est ni une transition confirmée ni nécessairement issue de Mario ; les appels directs qui ne passent pas par ce wrapper échappent à la sonde.**

Le diagnostic affiche toutes les 120 présentations, durant 32 fenêtres maximum, le nombre de lectures HID, copies, échantillons valides, fronts montants/descendants de boutons et demandes de statut. Les premières et dernières transitions d'une fenêtre et les premières/dernières demandes de statut portent les compteurs CPU `Present#` et `Rendu#` ainsi que leur masque ou code brut. Ces compteurs n'identifient pas une image visible ni une latence causale. Pas de logs individuels depuis les hooks, seuls les résumés sont écrits au moment de la présentation. Les accès aux états sont conditionnés à des pointeurs non nuls et à `state+0xB8`. Quatre hooks restent soumis à une garde de signature Ghidra.

**Statut : code préparé pour la PR suivante, non compilé et non testé en jeu.** Avant d'interpréter les résultats, vérifier d'abord que le champ de boutons change sur de vrais sauts en entraînement, et que la sonde de statut est appelée lors de ces actions. Si l'un des deux ne réagit pas, corriger l'instrumentation plutôt que de déduire un délai. Ne pas fusionner ; préserver les sauvegardes et plugins tiers.
