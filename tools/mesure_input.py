"""Mesure comparative manette -> changement visuel (Windows).
Ne modifie ni le jeu ni Ryujinx.
"""
import argparse
import csv
import os
from collections import deque
from datetime import datetime
from pathlib import Path
from statistics import median
from time import perf_counter_ns, sleep

FIELDS = ("version", "essai", "detecte", "latence_ms", "pas_capture_ms",
          "bruit", "seuil", "zone", "manette", "bouton", "horodatage")


def percentiles(vals):
    a = sorted(vals)
    def pct(p):
        at = (len(a) - 1) * p
        i = int(at)
        return a[i] + (a[min(i + 1, len(a) - 1)] - a[i]) * (at - i)
    return pct(.1), pct(.5), pct(.9)


def afficher_resultats(rows, name):
    vals = [float(r["latence_ms"]) for r in rows
            if str(r.get("detecte")) == "1" and r.get("latence_ms")]
    print(f"{name} : {len(vals)}/{len(rows)} essais detectes")
    if not vals:
        return
    p10, p50, p90 = percentiles(vals)
    print(f"  p10={p10:.2f} ms, mediane={p50:.2f} ms, p90={p90:.2f} ms")
    print(f"  mediane en frames a 60 FPS : {p50/16.6667:.2f}")
    steps = [float(r["pas_capture_ms"]) for r in rows
             if str(r.get("detecte")) == "1" and r.get("pas_capture_ms")]
    if steps:
        step = median(steps)
        print(f"  intervalle entre captures : {step:.2f} ms")
        if step > 12:
            print("  ATTENTION : acquisition lente, resolution faible pour un gain d'une frame")


def comparer(paths):
    series = []
    for path in paths:
        with open(path, encoding="utf-8-sig", newline="") as f:
            rows = list(csv.DictReader(f))
        name = rows[0]["version"] if rows else str(path)
        afficher_resultats(rows, name)
        vals = [float(r["latence_ms"]) for r in rows
                if str(r.get("detecte")) == "1" and r.get("latence_ms")]
        if len(vals) >= 15:
            series.append((name, median(vals)))
        else:
            print("  Moins de 15 detections valides : resultat peu exploitable")
    if len(series) > 1:
        name, baseline = series[0]
        for other, value in series[1:]:
            delta = value - baseline
            print(f"  {other} vs {name} : {delta:+.2f} ms ({delta/16.6667:+.2f} frame(s))")
    print("Mesure comparative : inclut aussi Ryujinx, Windows et la capture d'image.")


def changed_fraction(image, baseline, pixel, stable=None):
    import numpy as np
    changed = np.max(np.abs(image.astype(np.int16) -
                                baseline.astype(np.int16)), axis=2) >= pixel
    if stable is not None:
        changed &= stable
    return float(np.mean(changed))


def stable_pixels(images, baseline, pixel):
    import numpy as np
    base = baseline.astype(np.int16)
    activity = np.stack([
        np.max(np.abs(image.astype(np.int16) - base), axis=2) >= pixel
        for image in images
    ])
    # Ignorer les pixels qui bougent deja pendant l'attente de Mario.
    return np.mean(activity, axis=0) <= .10


def capture(sct, zone):
    import numpy as np
    image = sct.grab(zone)
    return np.frombuffer(image.bgra, dtype=np.uint8).reshape(
        image.height, image.width, 4)[:, :, :3].copy()


def region(taille, spec):
    if spec:
        numbers = [int(x.strip()) for x in spec.split(",")]
        if len(numbers) != 4 or min(numbers[2:]) < 40:
            raise ValueError("--zone = x,y,largeur,hauteur, minimum 40px")
        return dict(zip(("left", "top", "width", "height"), numbers))
    import ctypes
    from ctypes import wintypes
    class Point(ctypes.Structure):
        _fields_ = [("x", wintypes.LONG), ("y", wintypes.LONG)]
    input("Place le curseur AU CENTRE de Mario, puis appuie sur Entree ici. ")
    p = Point()
    if not ctypes.windll.user32.GetCursorPos(ctypes.byref(p)):
        raise RuntimeError("Position de la souris inaccessible")
    return dict(left=p.x - taille // 2, top=p.y - taille // 2,
                width=taille, height=taille)


def joystick_init(index, button):
    os.environ["SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS"] = "1"
    os.environ["PYGAME_HIDE_SUPPORT_PROMPT"] = "1"
    import pygame
    pygame.display.init()
    pygame.display.set_mode((1, 1), pygame.HIDDEN)
    pygame.joystick.init()
    if index >= pygame.joystick.get_count():
        raise RuntimeError("Manette absente dans SDL/Pygame. Pas de modification de Ryujinx.")
    pad = pygame.joystick.Joystick(index)
    pad.init()
    print(f"Manette detectee : {pad.get_name()} ({pad.get_numbuttons()} boutons)")
    if button is None:
        print("Appuie puis relache une fois le bouton de SAUT pour l'identifier.")
        previous = [0] * pad.get_numbuttons()
        while True:
            pygame.event.pump()
            current = [pad.get_button(i) for i in range(len(previous))]
            changed = [i for i, (a, b) in enumerate(zip(current, previous))
                       if a and not b]
            if changed:
                button = changed[0]
                break
            previous = current
            sleep(.01)
        print(f"Bouton de saut = {button}")
        while True:
            pygame.event.pump()
            if not pad.get_button(button):
                break
            sleep(.01)
    if not 0 <= button < pad.get_numbuttons():
        raise ValueError("Index de bouton invalide")
    return pygame, pad, button


def sauver(rows, label, sortie):
    path = Path(sortie) if sortie else (
        Path("mesures") / f"{label}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.csv")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(rows)
    print(f"CSV enregistre : {path.resolve()}")


def mesurer(args):
    import numpy as np
    import mss
    pygame, pad, button = joystick_init(args.manette, args.bouton)
    zone = region(args.taille, args.zone)
    print(f"Zone : {zone}. Garde cette MEME zone pour chaque version avec --zone.")
    print("Place Mario immobile sur fond fixe, sans menu ni autre personnage mobile.")
    print("Reviens sur Ryujinx ; saute une fois par essai, toutes les 2-3 secondes.")
    print("Debut dans 4 secondes. Ctrl+C pour interrompre et sauver le CSV.")
    sleep(4)
    rows = []
    recent = deque(maxlen=12)
    last_pressed = False
    pending = None
    ready_at = perf_counter_ns() + 1_000_000_000
    last_capture_ns = None
    limit_ns = int(args.timeout * 1e9)
    pause_ns = int(args.repos * 1e9)
    timestamp = datetime.now().isoformat(timespec="seconds")

    with mss.mss() as sct:
        try:
            while len(rows) < args.essais:
                pygame.event.pump()
                pressed = bool(pad.get_button(button))
                sampled_ns = perf_counter_ns()
                image = capture(sct, zone)
                captured_ns = perf_counter_ns()

                if pending is not None:
                    fraction = changed_fraction(image, pending["base"], args.pixel, pending["stable"])
                    if fraction > pending["threshold"]:
                        if pending["first_hit"] is None:
                            pending["first_hit"] = (captured_ns, last_capture_ns or captured_ns)
                            pending["hits"] = 1
                        else:
                            pending["hits"] += 1
                    else:
                        pending["first_hit"] = None
                        pending["hits"] = 0

                    timeout = captured_ns - pending["start_ns"] >= limit_ns
                    if pending["hits"] >= 2 or timeout:
                        found = pending["hits"] >= 2
                        hit, prev = pending["first_hit"] if found else (None, None)
                        row = {
                            "version": args.version, "essai": len(rows) + 1, "detecte": int(found),
                            "latence_ms": f"{(hit-pending['start_ns'])/1e6:.3f}" if found else "",
                            "pas_capture_ms": f"{(hit-prev)/1e6:.3f}" if found else "",
                            "bruit": f"{pending['noise']:.5f}",
                            "seuil": f"{pending['threshold']:.5f}",
                            "zone": f"{zone['left']},{zone['top']},{zone['width']},{zone['height']}",
                            "manette": pad.get_name(), "bouton": button,
                            "horodatage": timestamp,
                        }
                        rows.append(row)
                        print(f"Essai {len(rows)}/{args.essais} : " +
                              (row["latence_ms"] + " ms" if found else "NON DETECTE"),
                              flush=True)
                        pending = None
                        recent.clear()
                        ready_at = captured_ns + pause_ns

                elif sampled_ns >= ready_at and not pressed:
                    recent.append(image)

                if pressed and not last_pressed and pending is None and (
                        sampled_ns >= ready_at and len(recent) >= 8):
                    base = np.median(np.stack(list(recent)), axis=0).astype(np.uint8)
                    stable = stable_pixels(recent, base, args.pixel)
                    stable_share = float(np.mean(stable))
                    noise = max(changed_fraction(im, base, args.pixel, stable)
                                for im in recent)
                    threshold = max(args.seuil, 2 * noise + .005)
                    if stable_share < .30 or threshold >= .25:
                        print(f"Zone difficile : {stable_share:.0%} de pixels stables, "
                              f"bruit={noise:.1%}. Centre la zone sur Mario, "
                              "loin des effets visuels.")
                        ready_at = sampled_ns + pause_ns
                        recent.clear()
                    else:
                        if not rows:
                            print(f"Calibration : {stable_share:.0%} pixels stables, "
                                  f"bruit={noise:.1%}, seuil={threshold:.1%}.")
                        pending = {"start_ns": sampled_ns, "base": base,
                                   "stable": stable, "noise": noise,
                                   "threshold": threshold, "hits": 0,
                                   "first_hit": None}
                last_pressed = pressed
                last_capture_ns = captured_ns
                sleep(.002)
        except KeyboardInterrupt:
            print("Interruption : sauvegarde des essais deja realises.")
        finally:
            sauver(rows, args.version, args.sortie)
            afficher_resultats(rows, args.version)
            pygame.quit()


def main():
    p = argparse.ArgumentParser(description="Mesure comparative manette -> image sur Windows, sans modifier Ryujinx.")
    p.add_argument("--version", help="Nom du build teste, par exemple pr5")
    p.add_argument("--essais", type=int, default=30)
    p.add_argument("--manette", type=int, default=0)
    p.add_argument("--bouton", type=int)
    p.add_argument("--taille", type=int, default=140)
    p.add_argument("--zone", help="x,y,largeur,hauteur ; pour les comparaisons")
    p.add_argument("--seuil", type=float, default=.015)
    p.add_argument("--pixel", type=int, default=25)
    p.add_argument("--timeout", type=float, default=.5)
    p.add_argument("--repos", type=float, default=2.)
    p.add_argument("--sortie", help="CSV de sortie")
    p.add_argument("--comparer", nargs="+", metavar="CSV")
    a = p.parse_args()
    if a.comparer:
        comparer(a.comparer)
        return
    if os.name != "nt":
        p.error("La capture est reservee a Windows ; l'analyse CSV marche partout.")
    if not a.version or a.essais < 1 or a.taille < 40 or not 0 < a.seuil < 1:
        p.error("--version et parametres valides requis")
    mesurer(a)


if __name__ == "__main__":
    main()
