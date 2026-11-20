# Mesure comparative du délai d'entrée

Outil Windows indépendant de Ryujinx : lit le bouton de saut via SDL puis détecte automatiquement le premier changement d'image autour de Mario. Il ne modifie ni le plugin, ni Ryujinx, ni les sauvegardes.

Nécessite Python 3 sur Windows (pas WSL). Installer une seule fois :

```powershell
py -m pip install pygame mss numpy
```

Dans GitHub Desktop, utiliser la branche `test/mesure-latence-logicielle` pour disposer de l'outil, ou copier `tools/mesure_input.py` depuis cette branche sans changer la branche de ton NRO. L'outil se lance depuis la racine du dépôt :

```powershell
py tools\mesure_input.py --version pr5 --essais 30
```

Il détecte le bouton de saut en demandant une pression, puis demande de placer le curseur sur le centre de Mario pour sélectionner une petite zone de capture. Revenir ensuite à la fenêtre Ryujinx. Mario doit être seul, à l'arrêt, sur un décor fixe (mode Commandes), avec caméra et réglages identiques. Effectuer une trentaine de sauts espacés de 2 à 3 secondes, toujours depuis un état immobile au sol. Chaque essai enregistre un délai, un indicateur de bruit et l'intervalle entre captures. Le fichier `mesures/pr5_date.csv` est créé automatiquement.

Pour les PR suivantes, répéter exactement le protocole avec `--version pr6`, puis `--version pr7`. Conserver la même zone pixel `--zone x,y,largeur,hauteur` : les coordonnées sont affichées au premier lancement. Comparer les CSV exacts obtenus avec :

```powershell
py tools\mesure_input.py --comparer mesures\pr5_exemple.csv mesures\pr6_exemple.csv mesures\pr7_exemple.csv
```

La commande donne les médianes, percentiles 10/90, intervalles de capture et différences en millisecondes/frames à 60 FPS. Une baisse de 16,67 ms correspond approximativement à une frame. **Les noms CSV de la commande sont des exemples à remplacer par les fichiers générés.**

Limites : l'outil mesure **l'arrivée du bouton dans Windows jusqu'au premier changement de pixels capturé à l'écran par Windows**, pas l'instant exact du clic physique et pas les seules étapes internes du moteur. La détection peut être trompée par les animations au repos, un décor mobile ou un effet de rendu. Elle dépend de la vitesse de capture, de l'animation de Mario et de la pile graphique Ryujinx/Windows. Une capture trop lente (intervalle >12 ms) ou moins de 15 détections rend la comparaison fragile. Ne conclure à une frame gagnée qu'avec des mesures répétées, mêmes réglages et sans artefact visuel. Pour isoler strictement l'input→simulation→présentation du jeu, il faudra une instrumentation corrélant un identifiant de frame entre ces trois étapes : ce n'est **pas encore** ce que fait ce script.

Ce programme n'injecte aucune commande, il observe uniquement la manette et les pixels ; il est indépendant des PR #5, #6 et #7. Son analyse CSV est testée sur données synthétiques ; **acquisition Windows/manette non encore validée en conditions réelles**.


## Correction de calibration après essais utilisateur

En mode Entraînement sur décor immobile, l'ancien script pouvait indiquer `Zone trop animee` à chaque saut : le calcul `4 × bruit` et le rejet à 25 % tenaient compte des animations au repos de Mario. La mesure ne démarrait pas (CSV avec `0/0`).

Le correctif exclut de la comparaison les pixels qui changent pendant la calibration **avant** le saut. Il indique le pourcentage de pixels stables, le bruit et le seuil retenu. Si moins de 30 % de la zone est stable, il refuse encore la mesure pour éviter un résultat trompeur. Cette détection visuelle reste exploratoire : la présence d'une caméra mobile ou d'effets peut provoquer des faux positifs. Aucun gain d'input lag ne doit être annoncé sans répétitions et contrôle de la cohérence des sauts.

Pour reprendre un petit essai, récupérer la dernière version de `test/mesure-latence-logicielle` dans GitHub Desktop (Fetch puis Pull), sans toucher au NRO, et relancer : `py tools\mesure_input.py --version pr7 --essais 5`. Le message `Calibration : xx% pixels stables` indique que la zone a passé le contrôle. Si aucune mesure n'aboutit, relever le pourcentage affiché avant d'ajuster l'outil.
