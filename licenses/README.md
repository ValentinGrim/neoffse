# Third-party notices

NeoFFSE ports code from MIT-licensed projects by KiameV and Meyti, and
documents save-format rules it learned from KiameV's editor. The cipher's
code goes back to public-domain reference code by Bram Cohen. The MIT
notices travel with the ported work, as the MIT license requires.

| File | Project | What NeoFFSE ports | NeoFFSE code derived from it |
|---|---|---|---|
| `kiamev-final-fantasy-pr-save-editor.txt` | [final-fantasy-pr-save-editor](https://github.com/KiameV/final-fantasy-pr-save-editor) | The Pixel Remaster save rules | `docs/formats/pixel-remaster.md` |
| `kiamev-ffpr-save-cypher.txt` | [ffpr-save-cypher](https://github.com/kiamev/ffpr-save-cypher) | The Rijndael-256 save cipher, key and IV | The `cipher` module of `neoffse-format-pr`; the vector generator in `tools/cipher-vectors/`, which runs the library |
| `meyt-py3rijndael.txt` | [py3rijndael](https://github.com/meyt/py3rijndael) | The Python Rijndael that ffpr-save-cypher is based on: round structure, CBC and zero padding | The `cipher` module of `neoffse-format-pr`, through ffpr-save-cypher |

## Provenance of the cipher

The Rijndael-256 code of `neoffse-format-pr` is a hand port of KiameV's
ffpr-save-cypher (Go). That library is based on Meyti's py3rijndael
(Python), whose README credits a gist by jeetsukumaran. The gist ports Bram
Cohen's Java reference code of April 2001 and describes it as public
domain. NeoFFSE computes its S-box and tables at compile time and expands
the key schedule at run time, where the Go library embeds both.

## ID tables

The script `tools/pr-tables/` generates them from the master data and message files of the user's own game install.
Square Enix owns that data, so the tables hold facts: IDs, names, kinds and caps.  
The repository holds no graphic or audio asset, no dialogue and no extracted file.
