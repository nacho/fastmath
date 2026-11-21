# Fast Math

A mental arithmetic game for GNOME. Answer as many questions as you can before the time runs out.

![GTK4](https://img.shields.io/badge/GTK-4-blue)
![License](https://img.shields.io/badge/license-GPL--3.0-green)

## How to Play

1. Pick the operations to practice: addition, subtraction, multiplication and division. Select several to mix them in the same round.
2. Choose a level and the time limit (two minutes by default).
3. Type each answer and press Enter. A round has up to 60 questions and ends when you answer them all or the time is up.
4. See how many answers you got right and how fast you were. Good scores go into a top-10 leaderboard for each combination of operations, level and time limit.

A wrong answer moves on to the next question and costs nothing. Press Escape to abandon a round.

## Levels

| Level | Addition and subtraction | Multiplication and division |
|-------|--------------------------|-----------------------------|
| 1     | up to 10                 | tables up to 5              |
| 2     | up to 20                 | tables up to 10             |
| 3     | up to 50                 | tables up to 12             |
| 4     | up to 100                | tables up to 15             |
| 5     | up to 1000               | tables up to 20             |

Divisions are always exact and subtractions never go below zero.

## Building

Fast Math is written in Rust using GTK 4.20 and libadwaita 1.8 or newer (GNOME 49 or newer, Ubuntu 26.04). On older distributions, build inside the GNOME Flatpak SDK, for example with GNOME Builder.

```sh
glib-compile-schemas data/
GSETTINGS_SCHEMA_DIR=data cargo run
```

With Meson:

```sh
meson setup builddir
meson compile -C builddir
meson test -C builddir
```

Flatpak:

```sh
flatpak-builder --user --install --force-clean _build-flatpak build-aux/io.github.nacho.fastmath.Devel.json
```

Set `G_MESSAGES_DEBUG=fastmath` to see the debug messages.

## Sound Sources

- Correct/wrong sound effects: [freedesktop sound theme](https://freedesktop.org/wiki/Specifications/sound-theme-spec/) (CC BY-SA)
- Quiz background music: [quiz game music loop BPM 90](https://freesound.org/people/portwain/sounds/220060/) by portwain (CC0)

## License

GPL-3.0-or-later
