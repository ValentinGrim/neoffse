# Third-party notices

NeoFFSE ports code from MIT-licensed projects by KiameV and Meyti, and documents save-format rules it learned from KiameV's editor.  
The cipher's code goes back to public-domain reference code by Bram Cohen.  
The MIT notices travel with the ported work, as the MIT license requires.

| File | Project | What NeoFFSE ports | NeoFFSE code derived from it |
|---|---|---|---|
| `kiamev-final-fantasy-pr-save-editor.txt` | [final-fantasy-pr-save-editor](https://github.com/KiameV/final-fantasy-pr-save-editor) | The Pixel Remaster save rules | `docs/formats/pixel-remaster.md` |
| `kiamev-ffpr-save-cypher.txt` | [ffpr-save-cypher](https://github.com/kiamev/ffpr-save-cypher) | The Rijndael-256 save cipher, key and IV | The `cipher` module of `neoffse-format-pr`; the vector generator in `tools/cipher-vectors/`, which runs the library |
| `meyt-py3rijndael.txt` | [py3rijndael](https://github.com/meyt/py3rijndael) | The Python Rijndael that ffpr-save-cypher is based on: round structure, CBC and zero padding | The `cipher` module of `neoffse-format-pr`, through ffpr-save-cypher |

## Provenance of the cipher

The Rijndael-256 code of `neoffse-format-pr` is a hand port of KiameV's ffpr-save-cypher (Go).  
That library is based on Meyti's py3rijndael (Python), whose README credits a gist by jeetsukumaran.  
The gist ports Bram Cohen's Java reference code of April 2001 and describes it as public domain.  
NeoFFSE computes its S-box and tables at compile time and expands the key schedule at run time.

## ID tables

The script `tools/pr-tables/` generates them from the master data and message files of the user's own game install.
Square Enix owns that data, so the tables hold facts:  
IDs, names, kinds and caps.  
The repository holds no graphic or audio asset, no dialogue and no extracted file.

## Fonts

The app embeds two fonts. Each file travels unmodified with its notice.
Fonts are not code and are not ported.

| Files | Project and author | Source | License | Notice file | Status |
|---|---|---|---|---|---|
| `assets/fonts/cornelia-sans/cornelia-sans.ttf` | Cornelia Sans, FontStruct, by EliDirkx98 | <https://fontstruct.com/fontstructions/show/1914648> | CC BY-SA 3.0, <http://creativecommons.org/licenses/by-sa/3.0/> | `assets/fonts/cornelia-sans/license.txt` | Redistributed unmodified |
| `assets/fonts/barlow/Barlow-Regular.ttf`, `assets/fonts/barlow/Barlow-SemiBold.ttf` | Barlow, The Barlow Project Authors (copyright 2017) | <https://github.com/google/fonts/tree/main/ofl/barlow> | SIL OFL 1.1, <https://openfontlicense.org> | `assets/fonts/barlow/OFL.txt` | Redistributed unmodified |

## Icons

The app draws its icons from one set. The files travel unmodified with the licence text.

| Files | Project and author | Source | License | Notice file | Status |
|---|---|---|---|---|---|
| `assets/icons/material-symbols/*.svg` | Material Symbols, Google | <https://github.com/google/material-design-icons> | Apache License 2.0, <https://www.apache.org/licenses/LICENSE-2.0> | `assets/icons/material-symbols/LICENSE` | Outlined, weight 300, fill 0, grade 0, 24 px. Redistributed unmodified, file names shortened |

## Portraits

The editor shows one sprite per Final Fantasy I class. The sprites are original drawings.

| Files | Project and author | Source | License | Notice file | Status |
|---|---|---|---|---|---|
| `assets/portraits/ff1-pr/*.png` | NeoFFSE, original sprites | Drawn for the project | CC0 1.0 Universal, <https://creativecommons.org/publicdomain/zero/1.0/> | `assets/portraits/LICENSE` | Original work, public domain dedication |
