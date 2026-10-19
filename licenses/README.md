# Third-party notices

NeoFFSE ports code and data from MIT-licensed projects by KiameV and
Meyti. The cipher's code goes back to public-domain reference code by Bram
Cohen. The MIT notices travel with the ported work, as the MIT license
requires.

| File | Project | What NeoFFSE ports |
|---|---|---|
| `kiamev-final-fantasy-pr-save-editor.txt` | [final-fantasy-pr-save-editor](https://github.com/KiameV/final-fantasy-pr-save-editor) | Pixel Remaster save rules and the ID tables of FF I to VI |
| `kiamev-ffpr-save-cypher.txt` | [ffpr-save-cypher](https://github.com/kiamev/ffpr-save-cypher) | The Rijndael-256 save cipher, key and IV |
| `meyt-py3rijndael.txt` | [py3rijndael](https://github.com/meyt/py3rijndael) | The Python Rijndael that ffpr-save-cypher is based on: round structure, CBC and zero padding |

## Provenance of the cipher

The Rijndael-256 code of `neoffse-format-pr` is a hand port of KiameV's
ffpr-save-cypher (Go). That library is based on Meyti's py3rijndael
(Python), whose README credits a gist by jeetsukumaran. The gist ports Bram
Cohen's Java reference code of April 2001 and describes it as public
domain. NeoFFSE computes its S-box and tables at compile time and expands
the key schedule at run time, where the Go library embeds both.
