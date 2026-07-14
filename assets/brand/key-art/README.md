# WyrmGrid promotional key-art candidates

This directory preserves the model-native candidate renders and reproducible
production treatments for WyrmGrid's master promotional artwork. Files under
`originals/` are immutable source artifacts: do not resize, recompress,
colour-correct, crop, or overwrite them. Generated production treatments belong
under `derivatives/`.

The candidates were generated through ChatGPT's built-in image-generation path
on 2026-07-14. The WyrmGrid application icon was used only as a palette and
geometric-rhythm reference. Selected artwork from Phobos Chemistry Pathways,
Phobos Industrial Pathology, and PhobosLib was used only as atmospheric
inspiration. No source artwork was copied into these renders.

## Original artifacts

| Candidate                     | File                                             | Dimensions | SHA-256                                                            |
| ----------------------------- | ------------------------------------------------ | ---------- | ------------------------------------------------------------------ |
| 01 · Atlas Nocturne           | `originals/candidate-01-atlas-nocturne.png`      | 1672 × 941 | `2B56B562F3C94F02B7FF8E93D6647A1457866DA67D61CE2B28F666349494231D` |
| 02 · The Cartographer's Forge | `originals/candidate-02-cartographers-forge.png` | 1672 × 941 | `A49A338D5AE824A13E005B39FB22F8B2E6603B5DF14A6E355B144A0C578CBC6D` |
| 03 · World in Motion          | `originals/candidate-03-world-in-motion.png`     | 1672 × 941 | `95101B8DDCA943222E9CBD9B48F75709503E2E79FF22F54A82B7AFD2537FC272` |

These checksums match the files emitted by the image-generation tool before
they were copied into the repository worktree.

## Candidate directions

### 01 · Atlas Nocturne

A global command-table panorama: civilian aircraft, a luminous world grid, and
gold route flows with a subtle coiling-wyrm rhythm. This is the most direct
expression of WyrmGrid Atlas and the broadest platform story.

### 02 · The Cartographer's Forge

A tactile aviation-intelligence workshop combining charts, brass tools, radios,
and a cyan-and-gold projected route table. This has the strongest Phobos house
character and the most distinctive independent-project personality.

### 03 · World in Motion

A civilian turboprop crossing predawn weather above an illuminated operational
network. This is the most aviation-forward and aspirational candidate.

## Production roles

- **Dark surfaces:** Candidate 01 · Atlas Nocturne.
- **Light or brighter surfaces:** Candidate 03 · World in Motion.
- **Archived only:** Candidate 02 · The Cartographer's Forge. It is intentionally
  excluded from production derivatives because its dense detail is vulnerable
  to small crops and downstream compression.

## Rebuilding derivatives

Install the narrowly scoped image dependency and run the deterministic builder:

```powershell
python -m pip install -r assets/brand/key-art/requirements.txt
python assets/brand/key-art/build_derivatives.py
```

The builder produces branded 1600 × 900 heroes and 1280 × 640 social cards for
the selected dark and light sources. It records dimensions, source associations,
and SHA-256 checksums in `derivatives/manifest.json`.

## Publication notes

- Add the real WyrmGrid logo and all typography as deterministic overlays.
- Do not ask an image model to redraw the mark or render final wording.
- Keep the originals text-free and independent of OnAir trademarks or trade
  dress.
- Human-review aircraft geometry, implied geography, and all small background
  details before publishing a selected derivative.
