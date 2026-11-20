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

## Essai actif alternatif — PR #5 (non compilé et non testé)

Suite au blocage de la PR #4, le nouveau code repart de la branche stable `test/ordre-pipeline` **sans reprendre l'ancienne mutation des champs**.

Le code machine à `0x710384FA10` a été relu intégralement (256 octets, Ghidra Smash en lecture seule, désassemblage AArch64 local) :
- en début de routine, si `state+0x1D28` est actif, elle sélectionne `state+0x1D30`, attend que les compteurs `+0x18A8` et `+0x18A0` du slot sélectionné se rejoignent, puis soumet des commandes graphiques ;
- en fin de routine, lorsque `state+0x1D18` est activé, elle sélectionne naturellement le slot opposé de `state+0x1D20` dans `state+0x1D30` ;
- la routine ne garantit pas que deux appels rapprochés sur le même cycle visuel soient autorisés : **hypothèse expérimentale, non prouvée**.

Nouveau hook `src/dispatch_ready.rs` à l'offset `0x384FA10` : effectue **toujours l'appel original d'abord**. Après 120 présentations de chauffe, il vérifie que le slot suivant est 0 ou 1, correspond à `index_production ^ 1`, possède entre 1 et 128 commandes, et que le lot est entièrement produit. Il tente alors **un deuxième appel à la routine originale**, au plus une fois par présentation, sans écrire directement dans les champs de sélection ni désactiver la VSync, ni modifier le nombre de buffers, ni accélérer le gameplay. Les messages `premiere tentative`, `premier retour OK`, `120 passages termines` distinguent entrée et retour de cet appel.

Ce n'est **ni une garantie de réduction d'une frame ni un mécanisme de récupération automatique en cas de blocage**. Une double soumission peut rester dangereuse si la routine manipule d'autres files synchronisées ou si le slot semble prêt avant la fin d'une étape GPU. Test court, hors ligne, comparer vitesse et affichage, arrêter à la première anomalie. La commande de déploiement préserve une sauvegarde de notre NRO précédent. La référence sûre reste `test/ordre-pipeline`. Le résultat d'un test en jeu détermine si on travaille ensuite sur l'index de rendu, et seulement si la soumission anticipée fonctionne.


## Validation initiale du flush prêt — 20 novembre 2026, 11:05

Ryujinx 1.1.1403, Smash 13.0.5, log `Ryujinx_1.1.1403_2026-11-20_11-05-53.log` : Skyline charge le plugin, installe le hook `0x384FA10` et affiche `Flush lot pret : installe, attente 120 presentations` (00:07.417). À 00:21.908 : `premiere tentative` suivie de `premier retour OK`. À 00:26.315 : `120 passages termines`. L'utilisateur confirme que le jeu démarre et qu'il peut contrôler Mario dans le mode Commandes, sans comportement anormal perceptible. Les messages `TerminationRequested` et `DequeueBuffer: Busy` arrivent uniquement à l'arrêt à 01:02.654. **La routine de flush est rappelée et rend la main, sans blocage observable durant ce test.** Ce journal ne prouve pas que la seconde invocation soumet de nouvelles commandes effectives ni qu'une texture plus récente est affichée. **Aucune frame de latence gagnée n'est démontrée.** Tester une modification de l'index de texture séparément, sur une nouvelle branche, sans changer le flush validé.


## PR #6 — essai d'index rendu à +1 avec flush validé (non testé)

Point de départ : la PR #5 est chargée et la double invocation de la routine de dispatch se termine 120 fois sans blocage pendant un test dans Commandes (Mario) sur Ryujinx 1.1.1403 / Smash 13.0.5. Cela ne prouve pas une frame gagnée, ni même une soumission anticipée visible.

Ghidra Smash, binaire 13.0.5, lecture seule : `0x710386B0F8` = `ldr w9, [x8,#0x30]` (`B9403109`) ; `0x710386B0FC` = `add w9,w9,#2` (`11000929`) ; `0x710386B100` = `mov w10, ...` (`5295556A`), puis `72B5554A`. Les instructions suivantes calculent un modulo 3 pour choisir l'index de texture. La version expérimentale remplace uniquement `add w9,w9,#2` par `add w9,w9,#1` (`11000529`) pour tenter d'aligner le tampon rendu avec une soumission avancée. La correspondance temporelle de ce buffer avec la présentation et l'input n'est **pas démontrée** ; possible image périmée, clignotement ou blocage.

Pour éviter le conflit hook/patch, la sonde `render_probe` est déplacée de `0x386B0FC` à `0x386B100` ; elle voit l'index **après l'addition** et avant modulo 3. Le `snapshot` passe désormais à `raw % 3`. Le patch n'est tenté que si le hook dispatch a passé sa garde de signature et si les quatre mots entourant l'instruction +2 correspondent au binaire attendu. Pas de modification de VSync, de triple-buffer, du pacer ou de la cadence du gameplay. Les quatre sondes de référence restent conservées ; leurs statistiques de delta ne mesurent pas directement l'input lag.

**Statut : code commité sur une branche séparée, non compilé et non testé en jeu.** En cas de glitch, jeu figé ou vitesse anormale, arrêter l'émulation et redéployer la PR #5 `test/flush-lot-pret` (référence active fonctionnelle) ; aucun changement de configuration de Ryujinx. Si fonctionnement normal, comparer la réactivité par vidéo au ralenti entre PR #5 et PR #6 avec mêmes contrôles et mêmes paramètres ; un log seul ne peut mesurer la latence bouton-écran.


## Test du patch index +1 — 20 novembre 2026, 11:16

Fichier log utilisateur : `Ryujinx_1.1.1403_2026-11-20_11-16-16.log` (Ryujinx 1.1.1403, Smash 13.0.5). `Index +1 : instruction remplacee (VSync inchangee)` et installation de la sonde déplacée à `0x386B100` à 00:07.393. `Flush lot pret : premiere tentative` et `premier retour OK` à 00:21.720, `120 passages termines` à 00:24.646. Rotation `mod3=[40,40,40]` avec `max_avant_mod=3`. La trace se prolonge jusqu'à l'arrêt à 01:13.429, sans erreur fatale attribuable au plugin. Les erreurs `TerminationRequested`/`DequeueBuffer: Busy` coïncident avec la fermeture, comme lors des essais précédents. **Conclusion : application du patch et exécution du flush validées par log ; stabilité visuelle, rythme du gameplay et gain de latence non encore confirmés par l'utilisateur ni par mesures.** Aucun nombre de frames gagnées ne peut être déduit de `mod3` ni du retour du flush. Comparaison A/B PR #5 vs #6 nécessaire avant validation fonctionnelle.

## Variante expérimentale +0 — PR #7

Branche `test/index-rendu-sans-decalage` issue de la PR #6, sans reprendre le patch de dispatch défectueux de la PR #4. À `0x710386B0FC` (Smash 13.0.5 ; base `0x7100000000`), Ghidra Smash vérifie l'instruction `add w9, w9, #2` (`0x11000929`). Nouvelle hypothèse isolée : remplacer cette instruction par `nop` (`0xD503201F`), soit `+0` avant le modulo 3. Tous les autres mécanismes sont inchangés par rapport à la PR #6 : flush « lot prêt », VSync active, trois textures, pacer et quatre sondes. Le patch vérifie les quatre mots adjacents avant écriture et relit l'instruction après application.

**Cette variante +0 n'est pas compilée, pas testée en jeu et son avantage en input lag n'est pas démontré.** Elle peut afficher un buffer périmé, répéter des images, provoquer des glitches ou bloquer. Le succès de la PR #6 ne garantit pas la sécurité de ce changement. Avant essai, il faut confirmer le bon affichage et la vitesse de la PR #6, puis comparer A/B à réglages identiques. En cas d'anomalie, fermer Ryujinx et revenir à `test/index-rendu-plus-un` (PR #6) ou `test/flush-lot-pret` (PR #5), toutes deux conservées. Aucun changement de configuration, mod tiers ou sauvegarde.


## Analyse lecture des inputs -> traitement -> rendu (Ghidra en lecture seule, 20 novembre 2026)

Cible inspectée : programme `main` AArch64 chargé dans Ghidra Smash, base `0x7100000000`. Tous les pointeurs ci-dessous sont des adresses Ghidra absolues ; soustraire la base pour obtenir un offset Skyline. Aucune mutation du projet Ghidra, du jeu ou du plugin.

### Lecture HID : chemin vérifié dans le code

- Lors de l'initialisation des contrôleurs, la boucle principale (`FUN_7103741F40`) passe par `0x71037427AC` (`nn::hid::InitializeNpad`), `0x71037427DC` (`SetSupportedNpadStyleSet`) et `0x71037427E8` (`SetSupportedNpadIdType`). Elle construit les objets de contrôleur via `FUN_7103666E30` à `0x710374285C`, puis les place notamment dans le tableau `0x7105339860..0x71053398A8` (10 emplacements).
- `FUN_7103667C90` (offset Skyline `0x3667C90` ; premiers mots `FC1A0FE8 A90167FA`) parcourt ces objets et appelle leur méthode virtuelle `+0x10` pour actualiser leur état. La boucle principale l'appelle à `0x710374C894` et `0x710374CAB8`. Cela ne prouve pas que les deux chemins sont empruntés à chaque frame ; plusieurs branches contrôlent leur exécution.
- L'objet interne du contrôleur créé par `FUN_7103666E30` utilise la table virtuelle `0x7105238EF0`. Son entrée `+0x10`, lue dans la table, pointe vers `0x7103667210` (offset Skyline `0x3667210`, premiers mots `D10183FF A9044FF4`). **Ghidra n'a pas défini cette routine comme fonction** ; sa lecture a été faite par hexdump Ghidra puis décodage des instructions AArch64 `BL`, sans modification du binaire :
  - `0x7103667234` appelle l'import `nn::hid::GetNpadStyleSet` (`0x71039C6F30`).
  - `0x7103667280` appelle `nn::hid::GetNpadState` à `0x71039C6F80`.
  - `0x71036672FC` appelle une autre surcharge à `0x71039C6FA0`.
  - `0x71036673A8`, `0x7103667400` et `0x7103667458` appellent trois autres surcharges à `0x71039C6FC0`, `0x71039C6FD0`, `0x71039C6FE0`.
  Ces branchements sont **confirmés par les octets des instructions**. Ils n'apparaissent pas dans les xrefs de fonctions de Ghidra parce que cette routine est non définie. En pratique, **`0x3667210` est le premier candidat sérieux pour une sonde passive de lecture des inputs**. Son exécution et sa fréquence sur Ryujinx restent à valider.

### Propagation et traitement du contrôleur

- `FUN_7103762EC0` (offset `0x3762EC0`) lit jusqu'à 10 états depuis `0x7105339860..98A8` et recopie notamment les champs `+0x08`, `+0x0C` (masques), les axes et d'autres données après conversion. Cette routine est appelée à `0x710375F054` par `FUN_710375F020`, elle-même appelée depuis la boucle principale à `0x710374C89C` et `0x710374CABC`, après `FUN_7103667C90` sur les chemins où celui-ci est exécuté. **C'est une copie/normalisation de contrôleurs, pas la preuve que le combattant a déjà exécuté son action.**
- `FUN_710337B690` (offset `0x337B690`, premiers mots `D10283FF FD001BE8`) lit ces mêmes structures globales et des masques de boutons, puis appelle `FUN_710337B400` avec divers indicateurs. Un appel est repéré à `0x7101EF7504` ; son contenant n'est pas défini comme fonction par Ghidra. Cela en fait un **candidat de traitement/mappage des commandes**, non encore vérifié comme point où un saut de Mario est effectivement accepté par la simulation.
- Les wrappers `app::lua_bind::ControlModule__get_trigger_impl` à `0x71020013B0`, `get_button_impl` à `0x71020013E0` et `get_pad_flag_impl` à `0x7102001620` délèguent à des méthodes virtuelles ; les wrapper eux-mêmes ne démontrent ni le moment de lecture physique ni une consommation exactement une fois par frame.

### Lien au pipeline d'affichage (partiel)

- Dans la boucle principale `FUN_7103747820`, un compteur `0x710593B460` est incrémenté à `0x710374CAAC` et `0x710374CBE8`. Il est un candidat pour dater les passages CPU, mais **pas encore confirmé comme compteur univoque de frames simulées**.
- Le gestionnaire `FUN_7103725030` (offset `0x3725030`) est appelé depuis plusieurs branches ; ce n'est pas une preuve directe de simulation d'un combat.
- La présentation passe par `FUN_71038701E0` (appelée à `0x7103748128`), qui fait tourner `state+0xF0..0xFC`. La sélection des textures à `0x710386B0FC` reste `(raw+2) % 3` dans le binaire d'origine. Ces indices cycliques ne sont **pas des identifiants uniques de frames**, et ne permettent pas à eux seuls de conclure sur le délai de rendu ou d'affichage.

### Décision technique

**Ne pas tenter de retirer une autre frame.** Prochaine expérience utile : un diagnostic **passif** fondé sur `0x3667210` (transition des boutons HID du joueur 1), `0x337B690` (mappage candidat), un compteur de simulation à confirmer, puis le rattachement d'un identifiant de frame de rendu à la présentation réelle. En premier lieu vérifier en jeu que ces sondes se déclenchent dans cet ordre et uniquement pour les contrôles visés, sans ralentir le jeu. Enregistrer en mémoire les événements et ne journaliser qu'en fin de lot pour limiter la perturbation de timing.

Comparer une **base de référence avec le diagnostic passif seul** aux variantes PR #5/#6/#7 avec le même diagnostic, afin d'isoler les effets des patchs. Ne pas confondre une différence de compteurs de CPU avec des frames réellement présentées. Si la corrélation entre une commande et la texture effectivement présentée reste impossible, le diagnostic devra être rapporté comme partiel, **pas** comme une mesure validée d'input delay.

État : **reverse engineering vérifié dans Ghidra (lecture seule)** pour les appels et les champs précisés ; **aucun hook de mesure interne compilé, exécuté ou validé sur Ryujinx**. Le test visuel Windows précédent était trop dispersé pour déterminer un gain de 1 frame. Ne pas utiliser le résultat comme preuve d'efficacité de la PR #7.
