# Fast Math

A mental arithmetic game for GNOME. Answer as many questions as you can before the time runs out.

![GTK4](https://img.shields.io/badge/GTK-4-blue)
![License](https://img.shields.io/badge/license-GPL--3.0-green)

## How to Play

1. Pick a mode. In **Arithmetic**, choose the operations to practice: addition, subtraction, multiplication and division. Select several to mix them in the same round. In **Comparison**, choose what to compare (whole numbers, decimals, fractions and calculations, alone or mixed) and say whether the left side is less than, equal to or greater than the right side.
2. Choose a level and the time limit (two minutes by default).
3. Type each answer and press Enter, or in comparisons press the <, = or > button or key. A round has up to 60 questions and ends when you answer them all or the time is up.
4. See how many answers you got right and how fast you were. Good scores go into a top-10 leaderboard for each combination of operations (or of comparisons), level and time limit.

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

In comparison mode:

| Level | Whole numbers          | Decimals                                | Fractions                         | Calculations                                 |
|-------|------------------------|-----------------------------------------|-----------------------------------|----------------------------------------------|
| 1     | up to 20: 14 ? 16      | tenths: 7.7 ? 7.8                       | same denominator: 2/5 ? 3/5       | a sum or difference and a number: 5 + 8 ? 14 |
| 2     | up to 100: 87 ? 96     | tenths and hundredths: 8.3 ? 8.25       | same numerator: 1/3 ? 1/4         | a calculation and a number: 7 × 8 ? 54       |
| 3     | up to 1000: 321 ? 197  | thousandths: 8.605 ? 8.61               | equivalent: 2/4 ? 1/2             | two calculations: 6 × 9 ? 50 + 4             |
| 4     | negatives: −37 ? −48   | negatives with hundredths: −4.07 ? −4.7 | different denominators: 2/3 ? 3/4 | two calculations, sums up to 100             |
| 5     | −1000 to 1000          | negatives with thousandths              | improper and mixed: 7/4 ? 1 3/4   | two calculations, sums up to 1000            |

Fractions are shown with the numerator over the denominator, with denominators up to 12. The calculations use the operands of arithmetic levels 1, 2, 2, 3 and 4.

Each answer comes up about a third of the time. Equal decimals are written differently (8.3 and 8.30), and many unequal ones test place value: 8.3 is greater than 8.25, and 3.04 is less than 3.4.

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
