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


## Résultat Ryujinx PR #10 — 20 novembre 2026, log 14-04-07

Log utilisateur `Ryujinx_1.1.1403_2026-11-20_14-04-07.log`, Smash 13.0.5 : les 4 hooks `lecture HID`, `actualisation`, `masque de boutons` et `demandes de statut` ont été installés. La collecte couvre 27 fenêtres sur 32 avant fermeture volontaire (~1 min 20 s), sans exception fatale imputable au plugin. Sur les fenêtres régulières de 120 présentations : HID=960, actualisation=120, copies=120, `boutons_valides=120`.

La sonde du premier slot `state+0x08` a enregistré des transitions d'appuis et de relâchements, avec plusieurs valeurs brutes (`0x20` surtout, mais aussi `0x04`, `0x02`, `0x08`). Dans les fenêtres 16 à 26, les appuis sur `0x20` apparaissent environ une fois par fenêtre, cohérents avec des tests de boutons espacés. Cela **valide la lecture des transitions de boutons**, mais l'association exacte de `0x20` à l'action « saut » reste à contrôler avec la configuration manette ; on ne peut pas déduire la latence.

La sonde `StatusModule__change_status_request_impl` ne remonte qu'une seule demande, code brut `4`, après `Present#737`/`Rendu#737`, fenêtre 7 ; **aucune demande observée pendant les fenêtres 16–26 contenant chacune un appui `0x20`**. Ce wrapper n'est donc pas un repère fiable de la consommation de l'input dans le combattant, et ne doit pas servir à calculer le délai bouton→simulation.

Suite : garder la sonde des boutons comme point de référence passif, rechercher dans Ghidra le **chemin réellement utilisé pour appliquer un saut au combattant**, puis n'instrumenter que l'état retenu avec validation en entraînement. Les indices `Present` et `Rendu` visibles sont des compteurs CPU, pas des frames GPU attestées. Ne pas modifier les textures/flush/VSync sur cette branche de diagnostic.

**Statut : PR #10 compilée, déployée et sondes observées en jeu selon le log ; seul le déclenchement des hooks est validé, pas la mesure d'input lag.**


## Analyse du saut dans Ghidra Smash — 20 novembre 2026, lecture seule

Programme inspecté : `main`, Smash 13.0.5 AArch64, base Ghidra `0x7100000000`. Aucun changement du programme ni du projet Ghidra. L'objectif est de trouver le **statut réellement appliqué** après une transition d'input, pas de compter des demandes parfois ignorées.

### Valeurs vérifiées directement dans le binaire

- `FIGHTER_STATUS_KIND_JUMP_SQUAT = 10 (0x0A)` : chaîne `0x7104414A82` enregistrée à `0x710210F7AC..F7BC`, où `w2=w23`. `w23` a été chargé à `#0xA` à `0x710210F52C` sans réécriture dans l'intervalle.
- `FIGHTER_STATUS_KIND_JUMP = 11 (0x0B)` : chaîne `0x710437E37A` enregistrée à `0x710210F7C0..F7D0`, où `w2=w24`. `w24` a été chargé à `#0xB` à `0x710210F544` sans réécriture dans l'intervalle.
- `FIGHTER_PAD_CMD_CAT1_FLAG_JUMP = 0x00100000` : chaîne `0x71042EA17B`, enregistrement `0x71021249C8..D8` avec `mov w2,#0x100000`.
- `FIGHTER_PAD_CMD_CAT1_FLAG_JUMP_BUTTON = 0x00200000` : chaîne `0x71043D201D`, enregistrement `0x71021249DC..EC` avec `mov w2,#0x200000`.
- `CONTROL_PAD_BUTTON_JUMP = 2` est un identifiant de bouton de la couche Control (chaîne `0x7104250BB0` enregistrée `0x7102101E24..34`), **pas** un bit du masque HID. Le `0x20` relevé dans les logs PR #10 est un masque brut du contrôleur ; son interprétation comme saut **n'est pas démontrée** par ces constantes.

### Chemin de commande / changement de statut identifié, mais indirect

- `app::lua_bind::ControlModule__get_command_flag_cat_impl` à `0x7102001630` (offset Skyline `0x2001630`) charge `BOMA+0x48` puis effectue un saut virtuel par `vtable+0x350` vers la méthode produisant les flags de commande. Le wrapper ne contient pas la logique de filtrage du saut.
- `app::lua_bind::StatusModule__status_kind_impl` à `0x7102087720` (offset `0x2087720`) charge `BOMA+0x40` et saute à la méthode virtuelle `+0x110` : **point d'accès à l'état courant**, qu'il faudrait observer sur le combattant concerné (et pas simplement sur un appel arbitraire).
- `StatusModule__status_kind_next_impl` à `0x7102087730` : accès virtuel `+0x118` à l'état suivant. `StatusModule__set_status_kind_interrupt_impl` à `0x7102087740` : écrit une intention d'interruption via la virtuelle `+0x120` (ne prouve pas qu'un état est effectivement appliqué).
- `StatusModule__change_status_request_from_script_impl` à `0x71020876E0` (offset `0x20876E0`) transfère une demande de statut, paramètre `w1`, par `vtable+0x50`. Ce chemin est **candidat non testé sur Ryujinx**.
- La PR #10 observe uniquement `change_status_request_impl` à `0x71020876C0` (virtuelle `+0x48`) : **1 seul événement code 4** pendant une session comportant de nombreux sauts. Il ne faut pas réutiliser ce hook comme « saut appliqué ».

### Limite structurelle de l'analyse du seul `main`

- `FUN_71033AC490` répartit les scripts de statut par combattant. Pour Mario, la branche `case 0` aboutit à `0x71033AC630`, qui saute vers `lua2cpp::create_agent_fighter_status_script_mario` à `0x71039C29B0`.
- `0x71039C29B0` est **un trampoline d'import**, qui lit une cible via `0x71052A03B8` et branche vers un symbole externe `0x7107559000`. Ghidra Smash ne fournit aucune instruction à cette cible externe dans le `main` actuel. On ne peut donc pas prétendre avoir localisé **le code effectif de décision du saut de Mario** dans ce seul binaire.
- Les noms et valeurs de constantes prouvent les identifiants disponibles dans le jeu, **pas le chemin exact exécuté ni la frame à laquelle le combattant les consomme**. Éviter de déduire la latence des seuls index de présentation ou des requêtes de statut.

### Prochain point technique à confirmer

Deux directions possibles, à valider séparément avant tout patch : (1) observer en jeu les passages effectifs `WAIT -> JUMP_SQUAT (10) -> JUMP (11)` du **même combattant**, avec son BattleObjectModuleAccessor identifié, et les comparer aux fronts d'appui déjà validés ; ou (2) analyser en lecture seule le module de scripts de statut réellement chargé pour Mario, s'il est disponible. Un simple hook du wrapper `status_kind` **à l'entrée** n'a pas accès à sa valeur de retour puisqu'il s'agit d'un saut terminal `br` ; ne pas le présenter comme une mesure de statut sans instrumentation complémentaire vérifiée.

**Résultat : le statut cible (10 puis 11) et le flag de commande saut par bouton (0x200000) sont identifiés dans le binaire 13.0.5, mais le point de transition réellement exécuté et sa correspondance avec une frame affichée ne sont toujours pas confirmés en jeu. Aucune nouvelle sonde ni modification de timing déployée à ce stade.**


## Analyse directe du fichier extrait `lua2cpp_common.nro` — 20 novembre 2026

Fichier reçu de l'utilisateur après extraction de `data.arc` avec Arc Explorer. **Analyse statique en lecture seule**, sans importer dans ni modifier le projet Ghidra. SHA-256 du NRO :
`43876f961a80e413370629d32c23af9de6e4cc26b448bab88d5433714a566097`.
En-tête `NRO0`, taille `0x813000` ; `MOD0` à `0x54506C` ; table de symboles dynamique et relocations présentes. **Tous les offsets ci-dessous sont relatifs à l'image du NRO**, non à l'image `main` de Smash déjà utilisée pour les hooks Skyline. La correspondance exacte de cette archive extraite avec la version réellement chargée sous Ryujinx 13.0.5 reste à vérifier : le NSP visible dans le chemin utilisateur porte `[v0]`, tandis que l'émulateur avait un update 13.0.5 actif lors des tests.

### Fonctions et code disponibles dans le NRO (confirmés par la table de symboles et le désassemblage)

- `L2CFighterCommon::sub_transition_group_check_ground_jump()` : `+0x534E0`, taille `0x1EC` (492 octets). Elle vérifie des conditions, appelle `sub_check_button_jump()` à l'adresse PLT `+0x531080` (`BL` à `+0x53544`), puis peut déclencher `L2CFighterBase::change_status(L2CValue,L2CValue)` par `BL +0x530250` aux adresses `+0x5359C` ou `+0x53610`. La fonction retourne un booléen encapsulé dans `L2CValue` : chemins à `+0x53630..+0x53638`. Elle constitue une étape **candidate de consommation de la commande de saut au sol**.
- `L2CFighterCommon::sub_check_button_jump()` : `+0x54590`, taille `0x11C` (284 octets). Lit un élément de table `L2CValue` à l'index `0x20`, le combine avec la constante `const_value_table__ +0x218` via l'opérateur `L2CValue::operator&`, puis interroge `app::lua_bind::WorkModule__is_enable_transition_term_impl` via la PLT `+0x530840` (appel à `+0x5461C`). Cela relie directement une commande de saut à la vérification de transition autorisée ; ne permet pas encore d'associer le masque HID brut `0x20` à une action de Mario.
- `L2CFighterCommon::status_pre_JumpSquat()` : `+0x21F6C0`, taille `0x110` (272 octets).
- `L2CFighterCommon::status_JumpSquat()` : `+0x2245C0`, taille `0xB0` (176 octets). Appelle `sub_status_JumpSquat_check_stick_lr_update` et `status_JumpSquat_common` (PLT `+0x5369F0` et `+0x536A00`) ; appelle ensuite `sub_shift_status_main` (PLT `+0x52FFA0`) afin d'installer la suite du traitement. C'est une **entrée d'état** plus pertinente que le seul wrapper `StatusModule__change_status_request_impl` de la PR #10.
- `L2CFighterCommon::status_JumpSquat_Main()` : `+0x224680`, taille `0x64C` (1612 octets). Contient plusieurs appels à `L2CFighterBase::change_status` (via la PLT `+0x530250`, notamment `+0x224794`, `+0x224854`, `+0x224980` et `+0x224B5C`). La cible réelle de chaque branche et le personnage visé doivent encore être corrélés.
- `L2CFighterCommon::status_pre_Jump()` : `+0x220090`, 216 octets.
- `L2CFighterCommon::status_Jump()` : `+0x172040`, 208 octets ; `status_Jump_Main()` : `+0x172A10`, 352 octets.

Les adresses PLT ci-dessus ont été **résolues à partir des relocalisations `R_AARCH64_JUMP_SLOT`** du NRO, et ne sont pas déduites uniquement des noms voisins affichés lors du désassemblage. La table globale `const_value_table__` utilisée par ces fonctions est un symbole externe importé ; sa position et ses valeurs à l'exécution dépendent du chargement.

### Interprétation, limites et suite

Le chemin statique **vérification du saut au sol → vérification du bouton et du terme de transition → `L2CFighterBase::change_status` → routines `JumpSquat` puis `Jump`** est désormais documenté. Ce graphe confirme que le wrapper `StatusModule__change_status_request_impl`, observé seulement une fois dans la PR #10, n'est **pas une couverture suffisante des sauts**. Il ne prouve cependant ni que tous les sauts empruntent cette séquence (autres scripts/modes/mouvements), ni le moment exact où Mario passe en état 10 puis 11, ni la frame GPU affichée.

**Avant d'installer de nouveaux hooks**, vérifier dans Ryujinx quel `lua2cpp_common.nro` et quelle base mémoire sont effectivement chargés. Les offsets NRO **ne doivent jamais être installés comme offsets `main` Skyline** ; le module est chargé séparément et potentiellement plus tard. Vérifier signatures des instructions, ordre de chargement, collisions avec Skyline/ARCropolis et éventuelles fonctions substituées par des scripts propres au combattant. Ensuite seulement préparer une nouvelle PR expérimentale, instrumenter les passages effectifs dans `JumpSquat` et `Jump` (en identifiant le bon combattant), et vérifier en jeu avant de parler de latence. Aucun patch de timing, déploiement ni fusion n'a été effectué durant cette analyse.


## Validation de l'identite du NRO de saut — branche `test/identite-nro-saut`

Fichier utilisateur `lua2cpp_common.nro` : SHA-256 `43876f961a80e413370629d32c23af9de6e4cc26b448bab88d5433714a566097`, en-tete `NRO0`, taille `0x813000`. L'en-tete donne le **Build ID du NRO extrait** : `13f5177a7e86f0150f1a22aa9e611eacd010397c` (12 octets nuls suivants). **Ce Build ID n'est pas le Build ID `main` du jeu** (`21450C647B8C5940ED5B68218EA128FD`) et ne prouve pas a lui seul que l'extraction RomFS correspond a la version des scripts charges sous Ryujinx.

Le diagnostic ponctuel `src/nro_probe.rs` est branche au hook de presentation deja teste (aucune nouvelle interception NRO). Toutes les 120 presentations, pendant 32 fenetres maximum, il appelle `nn::ro::LookupSymbol` pour chercher les **symboles exports** :
- `_ZN7lua2cpp16L2CFighterCommon16status_JumpSquatEv`, offset extrait `+0x2245C0`, debut `D10143FF A9034FF4` ;
- `_ZN7lua2cpp16L2CFighterCommon11status_JumpEv`, offset extrait `+0x172040`, debut `D10183FF A9044FF4`.

Si l'un des symboles n'est pas disponible, il attend d'autres presentations. Sinon il exige que les deux adresses aient les **mêmes offsets relatifs à une base commune** que dans le fichier extrait, puis il vérifie les deux signatures AArch64, la magie `NRO0` à `base+0x10` et le Build ID à `base+0x40`. Il écrit un seul résultat `Build ID identique=true/false`, ou un message de divergence, sans modifier le code en mémoire. Les lectures de l'en-tête ne sont faites qu'après validation de la structure relative et des deux prologues.

**Pourquoi ne pas utiliser `skyline::nro::add_hook` :** cette interface requiert `nro-hook-plugin`, qui ne figure pas parmi les plugins chargés dans le dernier log Ryujinx (seuls ARCropolis et notre NRO sont listés). Eviter pour cette étape une nouvelle dépendance ou un conflit potentiellement destabilisant. L'appel standard `nn::ro::LookupSymbol` sert ici uniquement au diagnostic, sans hooks du module.

Cette vérification montre au mieux que le module actif a les mêmes offsets/prologues et le même Build ID que le fichier fourni. **Elle ne prouve pas encore que les transitions 10/11 sont exécutées sur le combattant testé**, ni qu'une frame GPU est affichée plus vite. La recherche à la présentation peut manquer un module non chargé ou déchargé (limite 3840 présentations) et peut nécessiter une autre stratégie. Aucun hook `A64InlineHook` n'est installé dans `lua2cpp_common`.

**Statut : PR de diagnostic préparée, non compilée, non testée sur Ryujinx, non fusionnée.** Pour le test initial, rester en mode Entraînement au moins jusqu'au message `NRO saut` indiquant `Build ID identique` ou `symboles indisponibles` ; partager les lignes correspondantes du log. Aucun test chronometre d'input lag n'est pertinent ici.


### Test Ryujinx de la PR #11 — 20 novembre 2026, log `Ryujinx_1.1.1403_2026-11-20_15-10-46.log`

**Validé en jeu sur Ryujinx 1.1.1403 / Smash v13.0.5** (log de lancement : application chargée à 00:00:00.916). Notre NRO et ARCropolis sont chargés ; le diagnostic commence à 00:00:07.265. À 00:00:21.808, au **premier contrôle après 120 présentations**, le log donne :
`[remove-the-frame] NRO saut : NRO0 valide ; Build ID identique=true ; base=0x667961A000 ; ID=[7A17F513,15F0867E,AA221A0F,AC1E619E,7C3910D0]`

Résultat : les deux symboles JumpSquat / Jump ont été résolus, leurs offsets relatifs et leurs prologues correspondent au NRO extrait, l'en-tête NRO0 est valide, et le Build ID correspond. L'incertitude sur **l'identité du module chargé dans cette exécution** est levée. La base `0x667961A000` est une adresse runtime liée à **cette seule exécution**, à ne pas figer dans le code.

Le jeu a ensuite produit 18 fenêtres d'input diagnostic avant fermeture, sans crash indiqué dans le log ; il n'y a eu **aucun hook NRO** dans cette PR. Le log signale `DequeueBuffer: Busy` à la fermeture, sans preuve de lien avec la sonde. Ce test ne valide ni l'interception des passages dans les routines JumpSquat / Jump, ni la correspondance avec Mario, ni une réduction de latence.

**Prochaine étape** : créer sur une **nouvelle branche de test** une sonde de comptage des entrées `status_JumpSquat` et `status_Jump`, après la même vérification d'identité du NRO actif, avec un journal synthétique par 120 présentations et sans modification du timing. Tout hook dans le NRO sera expérimental jusqu'à validation sur Ryujinx.


## Sonde experimentale de JumpSquat — PR #12, branche `test/observer-jump-squat`

Part du diagnostic validé en jeu par la PR #11. Après résolution et vérification de **deux symboles**, des **deux prologues**, de `NRO0` et du **Build ID `13f5177a7e86f0150f1a22aa9e611eacd010397c`**, la sonde appelle une fois `skyline::hooks::A64InlineHook` à l'adresse runtime de `L2CFighterCommon::status_JumpSquat()`. **Aucun offset NRO n'est traité comme offset `main`.** L'API est fournie par `skyline 0.6.0` et ne requiert pas `nro-hook-plugin` (vérifié dans les sources de `ultimate-research/skyline-rs`, `src/hooks.rs`). L'interception patchera le début de la fonction en mémoire pour observer son entrée : elle n'est donc pas aussi passive qu'une simple lecture et peut provoquer un crash ou un conflit ; le fonctionnement demeure **hypothétique jusqu'au test Ryujinx**.

Le callback `extern C` enregistre en atomiques les comptes d'entrée, la première et la dernière position `Present#` / `Rendu#` des appels par fenêtre de 120 présentations, et le pointeur `x0` (contexte candidat de `L2CFighterCommon`) **sans déréférencer ce dernier**. Aucun `println!` dans le callback, aucun changement de statut ni de timing. L'horodatage reste **uniquement CPU** ; il ne mesure pas la latence physique. La collecte cesse à la présentation 3840 ; le hook installé n'est pas retiré à chaud. Installer un hook dans un NRO déjà chargé, notamment au moment de la présentation, est une opération expérimentale susceptible de concurrencer un autre thread : **surveiller les logs et ne pas supposer une compatibilité automatique avec Ryujinx**.

Ne pas poser de hook `status_Jump` dans cette étape : valider d'abord le premier hook. Test prévu : démarrer sur branche de test, vérifier `[And64InlineHook] ... successfully!`, aller en Entraînement avec Mario et effectuer une dizaine de sauts espacés. Dans le log, chercher `JumpSquat diag` (`entrees > 0`) et les `Boutons joueur1`. Un nombre non nul confirmerait que la routine commune est traversée dans ce scénario **sans attribuer encore avec certitude l'appel au seul Mario**. Si le jeu plante, conserver le log, revenir à la branche #11 et analyser le crash avant tout nouvel essai.

**Statut actuel : code préparé, non compilé et non validé sur Ryujinx. PR brouillon, ne pas fusionner.**


### Validation en jeu PR #12 — log `Ryujinx_1.1.1403_2026-11-20_15-10-46.log` (nouveau test ajouté au même fichier)

Attention : le fichier communiqué **concatène deux exécutions** de Ryujinx. Les données suivantes concernent **la seconde exécution**, à partir du second démarrage `00:00:00.005`, et non la première session de la PR #11 qui figure encore en tête.

- Smash v13.0.5, Ryujinx 1.1.1403, Skyline + ARCropolis + notre NRO chargés. À `00:00:17.819`, validation complète `NRO0 valide ; Build ID identique=true ; base=0x48B6B97000`.
- À cet instant, `A64InlineHook` installé avec confirmation explicite : `inline hook 0x48b6dbb5c0->0x84dc33c successfully! 20 bytes overwritten`. **Hook mémoire JumpSquat validé pour cette exécution**.
- 31 fenêtres de 120 présentations (`Present#120` à `#3720`). **13 entrées `status_JumpSquat`** observées entre `Present#1785` et `#3127`, dans 12 fenêtres non nulles (une fenêtre contient deux entrées). Aucune des entrées dans les premières fenêtres ne permet d'inférer l'absence de saut avant la mise en place du hook ou hors de cette routine.
- Plusieurs correspondances ponctuelles exactes entre masque `0x00000020` dans `Boutons joueur1` et le compteur `Present#` d'entrée `JumpSquat` : `#1785`, `#1903`, `#2007`, `#2366`, `#2484`, `#2597`, `#2704`, `#2819`, `#2924`, `#3025` et `#3127`. Pour `#2160` et `#2258`, la fenêtre comportait deux appuis du même masque. Le contexte brut `x0` capturé vaut systématiquement `0x11741D7A00` pour ces 13 entrées ; il n'a pas été déréférencé ni attribué formellement à Mario.
- Le log va jusqu'à `Present#3720`, puis montre une fin de session ordinaire (avec les avertissements habituels `TerminationRequested` et `DequeueBuffer: Busy`) ; pas d'indice de crash lié au hook pendant ce test.

**Conclusion vérifiée en jeu :** l'interception de `L2CFighterCommon::status_JumpSquat` fonctionne sur Ryujinx dans cette configuration et le masque d'appui `0x20` est fortement associé aux appels de préparation du saut. **Ne pas conclure que délai d'input lag = zéro** : les compteurs `Present#` et `Rendu#` sont de simples jalons CPU lus par deux callbacks, sans horodatage sous-frame ni corrélation avec l'image affichée. Enregistrer des index égaux n'établit ni simultanéité effective ni absence de latence. Le hook `status_Jump` n'est pas encore installé/testé ; la comparaison vanilla/optimisé reste à faire. Aucun gain de frame mesuré.

**Suite minimale :** sur une nouvelle branche expérimentale, ajouter un hook `status_Jump` protégé par la même validation NRO, avec capture des transitions par appui sous forme d'une séquence d'événements (plutôt que seulement première/dernière entrée de fenêtre), puis déterminer si l'observation des passages `JumpSquat` et `Jump` permet des mesures CPU exploitables. Ne pas fusionner les PR sans autorisation.
