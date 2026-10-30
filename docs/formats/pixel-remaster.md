# Pixel Remaster save format

A Final Fantasy I to VI Pixel Remaster (PR) save file is compact JSON, compressed with raw DEFLATE,
encrypted with Rijndael-256 in CBC mode and written as base64 text.  
The container rules and the top-level JSON rules below hold in all six games.

## Scope and sources

The rules rest on 32 Steam save files the game wrote: 6 each for FF I and FF II, 5 each for FF III to VI.
Two more files, slot 2 of FF I and FF II that KiameV's editor wrote, show how that editor encodes ([Slot 2 of FF I and FF II](#slot-2-of-ff-i-and-ff-ii)).  
The content rules that the early-game FF III to VI saves do not exercise (ability offsets, FF VI espers and parties) come from the code of KiameV's Go editor and stay [C].

The FF I domain rules ([ID spaces and tables](#id-spaces-and-tables), [Equipment slots](#equipment-slots)) rest on two independent game states:
slot 1 and the Quick Save, four promoted characters each, from late game.  
The Auto Save equals slot 1 in every field those rules measure, and FF I slot 2 is the editor's copy of slot 1 [M].
Every [M] on an FF I character value in those sections rests on 8 character states.
The ranges quoted there are observations, not bounds: no measured value comes from a new game (level 1, bare hands, low gil).
Two in-game tests on edited copies of one FF I slot add readings for the level and the level-up ([Level-up in FF I](#level-up-in-ff-i)).
Their start states are NeoFFSE edits, not game states, so they stay out of the 8 and carry the tag [M, edited copy].
The names and IDs of FF I items, equipment, abilities, jobs, commands, maps and monsters, and the bounds the game states, come from the game's own master data, read from one install of FF I ([Game master data](#game-master-data)).

### Games

|   Game    | Steam app ID | Game folder in `My Games` |       Executable        |
|-----------|--------------|---------------------------|-------------------------|
| FF I PR   |   1173770    |   `FINAL FANTASY PR`      | `FINAL FANTASY.exe`     |
| FF II PR  |   1173780    |   `FINAL FANTASY II PR`   | `FINAL FANTASY II.exe`  |
| FF III PR |   1173790    |   `FINAL FANTASY III PR`  | `FINAL FANTASY III.exe` |
| FF IV PR  |   1173800    |   `FINAL FANTASY IV PR`   | `FINAL FANTASY IV.exe`  |
| FF V PR   |   1173810    |   `FINAL FANTASY V PR`    | `FINAL FANTASY V.exe`   |
| FF VI PR  |   1173820    |   `FINAL FANTASY VI PR`   | `FINAL FANTASY VI.exe`  |

The Go editor holds the first three columns in `ui/forms/io/savedir.go` and `saveSelect.go` [C].
Save folders copied from Proton installs confirm all six folder names: `compatdata/<app ID>/…/My Games/<game folder>/Steam/<SteamID64>` [M].
The `installdir` key of each game's `appmanifest_<app ID>.acf` equals the game folder, so the install folder under `steamapps/common/` carries the same name.
Each install folder holds the executable above and one `<name>_Data` folder [M].

Platforms covered:

- **Steam PC**: Windows native and Linux through Proton, covered from [Files on disk](#files-on-disk) to [Known pitfalls](#known-pitfalls).
- **PlayStation `slotN.sav`**: the variant the Go editor opens ([PlayStation variant](#playstation-variant)).
  Its rules come from the Go code: no measured file backs them.

### Sources

| Source | Location | Used for |
|---|---|---|
| KiameV's Go editor | `final-fantasy-pr-save-editor` 1.3.1, fork branch `bestiary-and-fixes`, commits `6f8f74a` to `d4de261` on top of `a91b889` | codec, data model, pitfalls (commit messages explain each bug) |
| Game master data | FF I PR, Steam app 1173770, build 20249397, Unity 2019.4.26f1, © Square Enix; bundles and files listed in [Game master data](#game-master-data); read with UnityPy 1.25.4 from copies of the bundles | IDs, kinds and English and French names of the FF I items, equipment, abilities, jobs, commands, maps and monsters, the base HP and experience tables, and the bounds in `game_constant_int.csv` ([Game master data](#game-master-data) and the four sections after it) [D] |
| Game binary | `GameAssembly.dll` and `global-metadata.dat` of FF I PR, build 20249397 (IL2CPP, Unity 2019.4.26f1), © Square Enix; copies read with `llvm-objdump` and own readers for the metadata tables; class and method names come from the metadata, rules from the machine code | the FF I level-up rule ([Level-up in FF I](#level-up-in-ff-i)) [B] |
| Go editor tables | `github.com/ValentinGrim/final-fantasy-pr-save-editor`, tag `v1.5.1` (`32d3e59`), MIT, © 2022 Travis Offtermatt: package `models/core/costs/ff1` and `models/nameValue.go`. Both are byte-identical to KiameV's last upstream commit `cea371fe99b4476e9ed419961eea04f7de605054` (repository archived), the original work | the FF I names the editor uses, compared with the game ([Go editor tables](#go-editor-tables)) |
| Cipher library | `github.com/kiamev/ffpr-save-cypher` v1.0.0, MIT, © 2024 KiameV, a port of py3rijndael | key, IV, padding |
| Real saves | one Steam save folder per game, read-only copies | every [M] fact about the files |
| Steam's own files | `libraryfolders.vdf`, `appmanifest_<app ID>.acf`, `remotecache.vdf`, `loginusers.vdf` and the install folders of one Linux Steam install with two libraries; structure only, personal values masked | every [M] fact about roots, libraries and Steam Cloud |
| steamlocate | [steamlocate](https://crates.io/crates/steamlocate) 2.1.1, MIT: `src/locate/linux.rs`, `src/locate/windows.rs`, `src/library.rs` | Steam roots, registry keys, library parsing |
| Steam on Flatpak | [Flathub `com.valvesoftware.Steam` wiki](https://github.com/flathub/com.valvesoftware.Steam/wiki) | Flatpak data path and library overrides |
| Proton prefixes | [GamingOnLinux, GE-Proton 9-3](https://www.gamingonlinux.com/2024/04/ge-proton-9-3-released-with-a-new-option-to-help-with-modded-games) | prefix in the game's library; Steam Deck session |
| Windows folders | Microsoft documentation: [`KNOWNFOLDERID`](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid), [`SHGetKnownFolderPath`](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetknownfolderpath), [OneDrive known folders](https://learn.microsoft.com/en-us/sharepoint/redirect-known-folders) | `FOLDERID_Documents`, OneDrive redirection |
| Steam Cloud roots | [SteamCloudFileManager `ROOT_PATH_MAPPING.md`](https://raw.githubusercontent.com/Fldicoahkiin/SteamCloudFileManager/master/ROOT_PATH_MAPPING.md), GPL-3.0; notes read, no code taken | root 2 is `WinMyDocuments` |
| In-game checks | edited files loaded in the game | every [G] fact |

The 34 measured files, sizes in bytes.
"Editor" marks a file that KiameV's editor wrote ([Slot 2 of FF I and FF II](#slot-2-of-ff-i-and-ff-ii)):

| File (first characters) | Role | FF I | FF II | FF III | FF IV | FF V | FF VI |
|---|---|---|---|---|---|---|---|
| `ookrbATY…` | slot 1 | 8 157 | 96 049 | 87 261 | 80 773 | 62 469 | 77 317 |
| `vgU2wnua…` | slot 2 | 7 855 (editor) | 95 835 (editor) | absent | absent | absent | absent |
| `7nCxyzTw…` | Auto Save | 112 305 | 63 749 | 88 197 | 80 005 | 61 533 | 35 589 |
| `Rl18osV3…` | Quick Save | 59 185 | 57 181 | absent | absent | absent | absent |
| `dp3fS2vq…` | bestiary | 689 | 561 | 177 | 177 | 177 | 221 |
| `2E4HxIyL…` | slot index | 133 | 133 | 133 | 133 | 133 | 133 |
| `V=SfwyU3…` | system settings | 861 | 861 | 861 | 861 | 861 | 861 |

### Confidence tags

| Tag | Meaning |
|---|---|
| [M] | Measured on the 32 game-written Steam files (FF I to VI) or, for roots, libraries and Steam Cloud, on one Linux Steam install; "[M, two editor files]" marks a measurement on the two editor-written files, and "[M, edited copy]" a measurement on a file the game wrote after a battle played on a copy edited with NeoFFSE (FF I, one slot) |
| [G] | Verified in game, including a menu read on an edited copy |
| [D] | Read from the game's own master data or message files (FF I, build 20249397); not checked on other builds or games |
| [B] | Read in the game's binary (FF I, build 20249397, IL2CPP), on a copy: class and method names come from the metadata, rules from the machine code; not checked on other builds or games |
| [C] | Taken from the code, tests or commit messages of KiameV's Go editor, or from the code or documentation of another source named beside the tag; not measured |
| [I] | Inferred from measurements, or a rule NeoFFSE applies; consistent with the data, not proven |

## Files on disk

A game keeps one save folder per Steam account.
The folder holds up to 22 slot files with fixed names, one bestiary shared by all slots, two more game files and one Steam file.

### Save folder

| Install | Folder |
|---|---|
| Windows | `<Documents>\My Games\<game folder>\Steam\<SteamID64>\` [C] |
| Linux, Proton, native Steam | `~/.local/share/Steam/steamapps/compatdata/<app ID>/pfx/drive_c/users/steamuser/Documents/My Games/<game folder>/Steam/<SteamID64>/` [M for all six games] |
| Linux, Proton, Flatpak Steam | the same path under `~/.var/app/com.valvesoftware.Steam/.local/share/Steam/` [C] |
| Any Steam library | `<library>/steamapps/compatdata/<app ID>/…`: the prefix can sit in a library other than the one that installs the game ([Steam roots and libraries](#steam-roots-and-libraries)) [M, C] |

- `<SteamID64>` is the 17-digit account ID, starting with `7656` [M].
  Each of the six `Steam/` folders holds one entry of that shape and no other [M].
  To list the accounts without reading a Steam file, keep the subfolders of `Steam/` whose names match `^7656\d{13}$` [M for the pattern on six folders].
  The rule works on both systems.
- With several account folders, the Go editor picks one that holds a slot file, then the most recent modification time [C].
  A scanner can list all of them and leave the choice to the caller [I].
- Under Proton the Windows user is `steamuser` (GE-Proton 11-9 and Proton 11.0-100) [M].
  `My Documents` there is a symlink to `Documents` [M].
- On Windows `<Documents>` is the Known Folder `FOLDERID_Documents`, which `SHGetKnownFolderPath` returns for the current user [C, Microsoft].
  OneDrive Known Folder Move redirects that folder to OneDrive [C], so the call returns the OneDrive path [I].
  The games run on Unity [M], which resolves "My Documents" through the shell, so the saves follow the redirection [I].
  The Go editor tries the Known Folder, then `%USERPROFILE%\Documents`, `%USERPROFILE%\OneDrive\Documents` and `%OneDrive%\Documents`, and drops duplicates after it resolves links [C].
  On a non-English Windows the last fallback may miss, because the OneDrive folder name can differ from `Documents`; the Known Folder call does not depend on that name [I].
- Steam Cloud confirms the Windows shape.
  Each `remotecache.vdf` lists every file of the save folder as `My Games/<game folder>/Steam/<SteamID64>/<file>` with `"root" "2"`, which a community table maps to `WinMyDocuments`, the `Documents` folder [M, C].

### Steam roots and libraries

On Linux, find the saves in three steps: collect the Steam roots, read every library each root lists, then probe `compatdata/<app ID>` in every library.
On Windows the save folder ignores Steam libraries and hangs off `<Documents>`.

**Roots.**
A candidate counts as a root only when it holds a `steamapps` folder [C, steamlocate]:

| Root | Install |
|---|---|
| `~/.local/share/Steam`, or `$XDG_DATA_HOME/Steam` when set | native; the real install [M] |
| `~/.steam/steam`, `~/.steam/root` | symlinks to `~/.local/share/Steam` on a native install [M] |
| `~/.steam/debian-installation` | Debian `steam-installer` [C, steamlocate] |
| `~/.var/app/com.valvesoftware.Steam/.local/share/Steam`, with `.steam/steam` and `.steam/root` beside it | Flatpak [C, Flathub wiki, steamlocate] |
| `~/snap/steam/common/.local/share/Steam`, with `.steam/steam` and `.steam/root` beside it | Snap [C, steamlocate] |

Three of the native candidates point at one folder [M].
Canonicalize each root and drop duplicates, or every save folder appears three times.
The Go editor and steamlocate both drop duplicates after they resolve links [C].
On Windows, steamlocate takes the root from the registry value `InstallPath` under `HKLM\SOFTWARE\Wow6432Node\Valve\Steam`, then under `HKLM\SOFTWARE\Valve\Steam` [C, steamlocate].
NeoFFSE reads no registry: install discovery takes `%ProgramFiles(x86)%\Steam` and counts it when it holds a `steamapps` folder [I].
Save discovery needs no root.

**Libraries.**
Read `<root>/steamapps/libraryfolders.vdf`.
Steam keeps a byte-identical copy at `<root>/config/libraryfolders.vdf`; the Go editor and steamlocate read the `steamapps/` one [M, C].
The file is text VDF, tab-indented, without a BOM [M]:

```
"libraryfolders"
{
	"0"
	{
		"path"		"<root>"
		"apps"
		{
			"1173770"		"<bytes>"
		}
	}
	"1"
	{
		"path"		"<library>"
	}
}
```

- The top key is `libraryfolders`.
  Each library is a child keyed `"0"`, `"1"`, and so on, and `"0"` carries the Steam root as its `path` [M, two libraries].
  The real file holds more keys per library (`label`, `contentid`, `totalsize`) than this sketch shows [M].
- `apps` maps an app ID to a byte count.
  For the six PR games the value equals `SizeOnDisk` in the game's `appmanifest_<app ID>.acf` [M].
  The lists matched the manifest files one to one on the measured install [M], but steamlocate lists apps from the manifests and notes that the VDF values may be stale [C].
- steamlocate keeps the children whose key parses as `u32` and reads `path` from each [C].
  On Windows the VDF string doubles each backslash and the Go editor replaces `\\` with `\` [C]; a VDF parser that unescapes strings does the same job [I].
- `<library>/steamapps/appmanifest_<app ID>.acf` exists in the library that installs the game [M].
  Its `AppState` block holds `appid`, `installdir`, `SizeOnDisk`, `LastOwner` (a SteamID64, personal data) and depot data [M].
  `installdir` is the game folder of [Games](#games).
  Save discovery does not need the install folder; an asset provider or a process match does.

**Prefixes outside the game's library.**
By default Steam keeps the prefix in the library that installs the game [C, GamingOnLinux]: on the measured install 41 of 43 prefixes belong to an app whose manifest sits in the same library, and no app ID has a prefix in two libraries [M].
Two cases break the rule:

- In the Steam Deck session (`-steamdeck`), Steam moves the default prefix path to the default install, so a game on an SD card or an external drive gets its prefix in the root library [C, GamingOnLinux].
  No Deck sample exists.
- Steam leaves a prefix behind when it uninstalls a game, so a save can outlive its install [M for one non-PR app, I for the PR games].
  The two prefixes without a manifest on the measured install are `compatdata/0` and the prefix of an uninstalled game [M].

The rule: for each app ID, probe `compatdata/<app ID>` in every library of every root, whatever library the manifest names, and report every hit [M, C].
The Go editor does this [C].
`STEAM_COMPAT_DATA_PATH` overrides the prefix path per launch [C, community guides]; a prefix moved that way lies outside every library and needs a folder the user picks.

**Flatpak libraries.**
The Flatpak app sees the host home under the same path but redirects `XDG_DATA_HOME` to `~/.var/app/com.valvesoftware.Steam/.local/share`; an extra library needs a `--filesystem` override and keeps its host path [C, Flathub wiki].
The `path` values of a Flatpak `libraryfolders.vdf` should resolve from the host [I].

### Slot files

The game names each slot file with a fixed 44-character string that looks like base64 and may contain `=` in the middle [C].
The JSON `id` key of a slot equals its slot number [M for 1 and 21 in six games and for 22 in FF I and FF II; C for the rest].

| Slot | PC file name | `id` | PlayStation file |
|---|---|---|---|
| Auto Save | `7nCxyzTwG31W3Zlg70mo751W8ETH1n+Km0dWOzRU84Y=` | 21 | `slot21.sav` |
| Quick Save | `Rl18osV3e9kPX9SMWQj8mqShFpTUmu1lf6Mb=FVVfqk=` | 22 | `slot22.sav` |
| 1 | `ookrbATYovG3tEOXIH4HqWnsv8TrUlRWzM8AlCmW2mk=` | 1 | `slot1.sav` |
| 2 | `vgU2wnuaPje2Or53Iqs8Mp=Al6sdM+GM04Iymv229Ow=` | 2 | `slot2.sav` |
| 3 | `uhHNR4g5QL5twqCc+IhexaltjtBjJnzzcxh5RBSy4G4=` | 3 | `slot3.sav` |
| 4 | `fmsBRQ+D6YzdjCbBbl7BQuagHyg=7iX3I=EnhccyGDM=` | 4 | `slot4.sav` |
| 5 | `NXa+MQ+hiHKlPAHJ6GiVWi2Wk5JR2xQQaQxzhyCbK2E=` | 5 | `slot5.sav` |
| 6 | `UWtRedIOaeA6ig=8r6DIvxg33X92oMM9P8JBwiag4d0=` | 6 | `slot6.sav` |
| 7 | `e1gfNt2iCE2I3yucQ8zfXn0ou+P2=lREb2q7Lqm04Gc=` | 7 | `slot7.sav` |
| 8 | `6Pf6Ky7e4QBPuKH9EFJ1Iu+BUEz0zNrXdaS8866Gcq0=` | 8 | `slot8.sav` |
| 9 | `9dHjN5+9JJWfJ9xoprXo=ehwoEwJwKRYL1Hlc92UNQk=` | 9 | `slot9.sav` |
| 10 | `oY6N7KlcC4jscZnfa4ea6Nr=TUSR+I=29kwPNZe2NAo=` | 10 | `slot10.sav` |
| 11 | `NKQ3ux2pea=DqE=vXPKb8+oix5Lt467opYaG0p0brgU=` | 11 | `slot11.sav` |
| 12 | `HyhjsKWa=tCVf3TWB3qRy7NyrJbc8orciJCntDpqT=I=` | 12 | `slot12.sav` |
| 13 | `hl9YCUf633k79xePC9PiKAEOq1ajUcSZkLofQuNw2OM=` | 13 | `slot13.sav` |
| 14 | `C=ozNkSxgKEoLCgOPLJakAUUhnL820LbGlpMz0irQFI=` | 14 | `slot14.sav` |
| 15 | `z2837SldCS+oIV8y4w5LrnJK9URKYy1QrnoA9bvCg5o=` | 15 | `slot15.sav` |
| 16 | `CnvUyfaDeqDg3XbVpVWJOj=sPKcGMCV3dR=xM8Ze5jE=` | 16 | `slot16.sav` |
| 17 | `eQ9Km3NT1WoE4h0hFD90ggFIZayYxfHkIVntc7akYVo=` | 17 | `slot17.sav` |
| 18 | `Lnbq+GaFOc4ybPZaCf=llI0arXo06rJL32Eu+mCwsLg=` | 18 | `slot18.sav` |
| 19 | `9GkO1xc52WAzswcEtJxs963MkuCohOHgYj0Fhio=fPE=` | 19 | `slot19.sav` |
| 20 | `mkYfUr4Mtg0zUmF=6lw+bxRLnbnBYp9ayg1KgploDpQ=` | 20 | `slot20.sav` |

The names come from `saveSelect.go` [C].
The Auto Save and slot 1 names appear in all six games, the Quick Save name in FF I and FF II [M].
The slot 2 name appears in the two editor-written files, and the game lists those files [G].
The FF III to VI folders hold no Quick Save file because the player made none in those games [I].
The names hold `+` and `=`, which are legal on Windows and Linux; quote them in shells.

**Which slot a file holds.**
NeoFFSE reads the slot from the JSON `id` key: 1 to 20 are numbered slots, 21 is the Auto Save and 22 the Quick Save.
When `id` is absent or names no slot, it falls back to the file name, matched byte for byte against the table [I].

### Other files in the folder

| File | Size (FF I / FF II / FF III / FF IV / FF V / FF VI) | Content |
|---|---|---|
| `dp3fS2vqP7GDj8eF72YKqbT7FIAF=e7Shy2CsTITm2E=` | 689 / 561 / 177 / 177 / 177 / 221 | Bestiary, shared by every slot of the game ([Bestiary](#bestiary)) [M] |
| `2E4HxIyL+Mvaahk1GRNNOgKrO4vDSSm72kHLbVt71hg=` | 133 in all six | Slot index: `{"keys":[1,…,22],"values":[22 booleans]}` [M] |
| `V=SfwyU31ksiHpTxO7Rg2aBJrA18kSKvtF4jvDB4hRE=` | 861 in all six | System settings, 18 keys ([Documents of the other files](#documents-of-the-other-files)) [M] |
| `steam_autocloud.vdf` | 53 / 53 (FF I, FF II) | Steam Cloud marker, plain-text VDF with one `accountid` key [M] |

- The three game files use the container of [Container encoding](#container-encoding) [M].
- **Slot index.**
  `values` is `true` at the slots whose file the game wrote and `false` at the others: keys 1, 21 and 22 in FF I and FF II, keys 1 and 21 in FF III to VI [M].
  The payload is 207 bytes in FF I and FF II and 208 in FF III to VI: one more `false` (5 characters) in place of a `true` (4) [M].
  A slot file written by another tool leaves the index unchanged [M, two editor files].
  The load menu lists such a file: slot 2 of FF I and FF II, flagged `false`, appears in the menu [G].
  The index does not control which slots the menu lists, and what the game reads it for is unknown.
  The Go editor never reads or writes it [C].
- **System settings.**
  The file is byte-identical in all six folders [M].
  The Go editor never touches it [C].
- **`steam_autocloud.vdf`.**
  Steam writes it, not the game.
  `accountid` equals `SteamID64 − 76561197960265728`, the account part of the folder name [M].
  Leave it alone.
  The FF III to VI folders hold one beside the saves [M]; its size was not measured.
  Its presence marks a folder that Steam Auto-Cloud syncs, so a tool may use it to warn about Cloud conflicts [I].
- **Steam Cloud.**
  `userdata/<account ID>/<app ID>/remotecache.vdf` under the Steam root lists every file of the save folder except `steam_autocloud.vdf`, with 7, 7, 5, 5, 5 and 5 entries for FF I to FF VI [M].
  Each key is relative to Cloud root 2, `WinMyDocuments` ([Save folder](#save-folder)).
  `<account ID>` is the 32-bit account ID: `SteamID64 − 76561197960265728` names an existing `userdata` folder [M].
  The same folder also holds `userdata/0` [M].

### Slot 2 of FF I and FF II

The FF I and FF II folders each hold a slot 2 file that KiameV's editor wrote, not the game.
Five measurements separate each from the 32 game files [M]:

| Evidence | Slot 2 (FF I, FF II) | The 32 game files |
|---|---|---|
| CRLF after the base64 | absent | present in 32 of 32 |
| Top-level `playTime` | `0` (integer form) | `0.0` in 14 of 14 slots |
| Dotted floats in the file | 1 (`userData.playTime`, non-integral) | 24 (FF I) and 33 (FF II) in the slots they copy |
| Slot index flag for slot 2 | `false` | matches the slot files present in 6 of 6 folders |
| Decoded content against slot 1 | equal except `id`, `userData.playTime` and, in FF II, 12 `skillLevel` values | |

Both streams end with a fixed-Huffman final block (`… 03 00` in FF I, `… 07 00` in FF II), and `Decrypt` strips that last zero byte: each file loses one stream byte ([Zero padding and the stripping trap](#zero-padding-and-the-stripping-trap)).
The BOM and the PNG thumbnail match slot 1.
The editor wrote slot 1 data under the slot 2 name with `id` set to 2 [I].

The two files show how the editor encodes [M, two editor files]:

- It writes the BOM back and no CRLF ([BOM and line end](#bom-and-line-end)).
- It writes integral floats as integers: `0`, not `0.0` ([Numbers](#numbers)).
- Its streams can end in a zero byte that `Decrypt` deletes.
- It sets `id` to the target slot number.
- It leaves the slot index untouched ([Other files in the folder](#other-files-in-the-folder)).

### PlayStation naming

The Go editor names PlayStation slots `slotN.sav`, with N taken from the `id` column of [Slot files](#slot-files): `slot1.sav` to `slot20.sav`, `slot21.sav` for Auto Save, `slot22.sav` for Quick Save [C].
It looks for the bestiary under the PC name `dp3fS2vq…` in the same folder [C].

## Container encoding

A PC file is base64 text, with a UTF-8 BOM in front and CRLF at the end, of a Rijndael-256-CBC ciphertext of a raw DEFLATE stream of compact UTF-8 JSON.
The reference decryption deletes trailing zero bytes that belong to the DEFLATE stream; a decoder must keep them.

### Layers

| Layer | Bytes | Tag |
|---|---|---|
| Prefix | UTF-8 BOM `EF BB BF` | [M] 34 of 34, editor files included |
| Text | one base64 line, standard alphabet, `=` padding | [M] |
| Line end | `0D 0A` | [M] all 32 game files |
| Ciphertext | Rijndael, 32-byte blocks, CBC | [M] |
| Plaintext | raw DEFLATE stream followed by 0 to 31 zero bytes | [M] |
| Payload | compact UTF-8 JSON, no BOM, no trailing newline | [M] |

The inflater stops at the final block and ignores every byte after it, zero or not.

### BOM and line end

- The game writes the BOM and the CRLF on every file [M, 32 files].
- The Go editor's decoder records the BOM as a prefix and trims trailing space, tab, CR and LF before base64 [C].
  Its encoder writes the BOM back and no CRLF [M, two editor files].
- The game loads an FF I bestiary written by the Go editor's encoder, so it accepts a file without the trailing CRLF [G].
- NeoFFSE trims the same four bytes and keeps them as the trailer, byte for byte, up to 64 bytes.
  A longer trailer fails decoding, and encoding refuses a container that holds one.
  Any other byte, a form feed included, stays in the base64 text, and decoding fails on it.
- `decode` returns the BOM flag and the trailer, and `encode` writes back the BOM and the trailer of the container it receives.
  `Container::game()` holds the BOM and the CRLF the game writes.
  A container decoded from an editor file holds the BOM and no CRLF, and `encode` keeps it that way.
  The FF I document keeps the container it decoded and encodes with it, so a save writes back the BOM and the trailer of the file it read.
  `Container::game()` serves a caller that builds a file from nothing.

### Base64

- RFC 4648 standard alphabet (`A-Z a-z 0-9 + /`) with `=` padding [M]: text lengths are multiples of 4, and the files end in `=`, `==` or no padding as the ciphertext length requires.
- Text length is `4 × ceil(len(cipher) / 3)`.
  Example: 96 bytes of ciphertext give 128 characters, plus 3 bytes of BOM and 2 of CRLF: the 133-byte slot index [M].
- The Go editor's decoder appends missing `=` before decoding [C].
  Before `6f8f74a` it ignored base64 errors and went on with empty data [C].
- NeoFFSE refuses a CR or LF inside the text.
  Go's `encoding/base64` decoder skips them, and no measured file holds one [M].
- NeoFFSE refuses a text longer than 44 739 244 bytes (`MAX_TEXT_LEN`) before it decodes.
  The largest measured file is 112 305 bytes.

### Cipher

| Parameter | Value |
|---|---|
| Algorithm | Rijndael, 256-bit block, 256-bit key, 14 rounds |
| Mode | CBC, fixed IV |
| Key (32 bytes, hex) | `61090700b9c3b8b9421599e79ca57b87bea932d3797bad7663ed4dde0a943cc5` |
| IV (32 bytes, hex) | `0224f9771fdb6e0e3bd508d7b795bf2e0cbd176942680a637b12bc6273db2ebb` |

- AES fixes the block at 128 bits, so an AES library cannot decrypt these files.
  NeoFFSE ports Rijndael-256 from the Go library.
- CBC: the encryptor XORs the first plaintext block with the IV and each later block with the previous ciphertext block [C].
- The Go library packs each 4-byte column big-endian (`b0<<24 | b1<<16 | b2<<8 | b3`) and ships both expanded key schedules (`ke` and `kd`, 15 round keys of 8 words) as constants.
  `ke[0]` is the key: `ke[0][0] = 1627981568 = 0x61090700` [C].
  Both schedules follow from the key, so an implementation may expand them at run time instead of embedding them.
- With a 256-bit block (Nb = 8), the rows of the state shift by 0, 1, 3 and 4 columns in encryption and by 0, 7, 5 and 4 in decryption.
  The Go library holds these as `shifts[2]` [C].
- One key and one IV serve every game, slot, bestiary and settings file [M: all 34 files decrypt with them].
- With a fixed key and IV, equal plaintexts give equal files: the settings file is identical in all six games, and the slot index file is identical in FF I and FF II and again in FF III to VI [M].

### Zero padding and the stripping trap

**Padding.**
The encryptor appends zero bytes up to the next multiple of 32 and adds none when the length is a multiple of 32 (`padder.Encode`) [C].
Game files carry 2 to 30 bytes of padding; none of the 32 carries 0 [M].

**Stripping.**
`rijndael.Decrypt` returns `padder.Decode(plain)`.
That function scans the last 31 bytes from the end and drops each zero byte until it meets a non-zero byte, so it deletes up to 31 bytes [C].
It cannot tell padding from data.
A raw DEFLATE stream often ends with `0x00` (for example the 7-bit end-of-block code of a fixed-Huffman block), and the function deletes that byte too.
Inflating the shortened stream fails with `unexpected EOF`.

Measured cases [M]:

| File | Ciphertext | DEFLATE stream | Padding | Bytes `Decrypt` removed | Stream bytes lost |
|---|---|---|---|---|---|
| FF II Quick Save | 42 880 | 42 854 | 26 | 27 | 1 |
| Slot index, FF I and FF II | 96 | 83 | 13 | 14 | 1 |
| FF I slot 2 (editor) | 5 888 | 5 875 | 13 | 14 | 1 |
| FF II slot 2 (editor) | 71 872 | 71 869 | 3 | 4 | 1 |
| The other 29 game files | | | | equal to the padding | 0 |

No FF III to VI file loses a stream byte, yet a decoder must keep every decrypted byte: the loss depends on the last stream byte.

**Fix.**
After `Decrypt`, append `len(cipher) − len(plain)` zero bytes, then inflate.
The inflater stops at the final block and ignores the zeros after it, as the game does [C, `codec.go`].
A port whose decryption never strips returns all `len(cipher)` bytes and needs no restore step.

**Fallback.**
A stream without a final block (sync-flushed only) runs into the padding, which an inflater reads as a stored block with a bad LEN/NLEN pair.
The Go editor's codec then inflates the stripped bytes and accepts the result only when inflation hit the end of the input and the output is valid JSON [C, `TestDecodeStreamWithoutFinalBlock`].
NeoFFSE accepts the stripped view when inflation consumed all of it without reaching a final block and the inflated bytes pass a shape check: they start with `{`, brackets balance outside strings, the depth first returns to zero at the last byte and no string stays open.
The check does not match a `[` with a `]`; full JSON validation belongs to the document layer.
None of the 34 files needs this path [M].

### Raw DEFLATE

- The stream is raw RFC 1951 DEFLATE: no zlib header, no gzip header, no checksum [M].
- Each stream ends with a block whose BFINAL bit is set, and holds no `00 00 FF FF` sync-flush marker or stored block [M, 34 files].
  Game streams use dynamic-Huffman blocks; the FF II Quick Save and the two editor files end with a fixed-Huffman final block [M].
- The game's compressor differs from zlib on large inputs.
  No tested zlib setting reproduces a slot or the settings file [M]: levels 0 to 9 with memLevel 8 or 9 on all files, and a grid of levels 1 to 9, memLevel 1 to 9, windowBits 9 to 15 and two strategies on FF III and FF VI.
- Small streams can match: the slot index (a 207-byte payload in FF I and FF II, 208 in FF III to VI) at level 9, the FF III to V bestiaries at levels 7 to 9 and the FF VI bestiary at level 7 [M].
  The FF I and FF II bestiaries match no setting.
- zlib does not reproduce the game's bytes, so a round-trip test compares the inflated JSON text.
- The game accepts a stream from Go's `compress/flate` at level 6, closed with a final block [G, FF I bestiary].
- JSON payloads of 80 to 235 KB compress to 6 to 84 KB in slot files [M].
- NeoFFSE caps the JSON text at 16 MiB (`MAX_PAYLOAD_LEN`) in both directions: decode refuses an inflated payload above it and encode refuses a longer text, so it never writes a file it would refuse to read.

### JSON text

The payload is compact JSON: no space or newline between tokens, no trailing newline [M, 34 files].
Strings hold raw UTF-8 (`é` as `C3 A9`), with no `\u` escape and no escaped `/` [M].
Only `\"` and `\\` escapes occur, and no raw control character [M].
[Top-level JSON document](#top-level-json-document) covers the document.

### Pseudo-code

Decode:

```text
decode(file) -> (json, container):
    bom = file starts with EF BB BF
    if bom: file = file[3..]
    text = file without trailing ' ', '\t', '\r', '\n'
    trailer = the bytes removed from the end         # kept as read, maybe empty
    require len(trailer) <= 64                       # NeoFFSE cap; the game writes 2
    while len(text) % 4 != 0: text += "="            # tolerance, the game pads
    cipher = base64_std_decode(text)                 # error: damaged file
    require len(cipher) > 0 and len(cipher) % 32 == 0
    plain = rijndael256_cbc_decrypt(cipher, KEY, IV) # len(plain) == len(cipher), no unpadding
    json = raw_inflate(plain)                        # stops at the final block
    if inflate failed:
        s = plain minus its trailing zero bytes (at most 31)
        json = raw_inflate(s)
        accept only if inflation consumed all of s without a final block
            and json is a balanced object (see Fallback)
    return (json, container = (bom, trailer))
```

Encode:

```text
encode(json, container) -> file:
    require len(json) <= 16 MiB                      # NeoFFSE cap, the one decode applies
    stream = raw_deflate(json)                       # ends with a BFINAL block
    padded = stream + zero bytes up to a multiple of 32   # none when aligned
    cipher = rijndael256_cbc_encrypt(padded, KEY, IV)
    text = base64_std_encode(cipher)                 # with '=' padding
    return (EF BB BF if container.bom) + text + container.trailer
        # the trailer is whatever the container holds: the game's container
        # holds CR LF, a container decoded from an editor file holds nothing
```

### PlayStation variant

The Go editor reads a `slotN.sav` file as the JSON text of [Top-level JSON document](#top-level-json-document), with no BOM, base64, cipher or DEFLATE, and writes it back the same way [C].
It reads the bestiary file the same way [C].
PS4 and PS5 encrypt saves with console keys ([Goals and scope](../architecture.md#goals-and-scope)), so these files must be what an external tool extracts from the console.

## Top-level JSON document

Paths in this document use `.` for an object member and `→` for a step into a JSON string.

A slot holds one object with ten keys.
Four of them (`userData`, `configData`, `dataStorage`, `mapData`) hold whole JSON documents serialized into strings, and those documents nest more of the same.
An editor parses a string document, edits it and re-serializes it in the game's style, and leaves each untouched string as it read it.

### Slot keys

In this order, in all 16 slot files of the six games [M]:

| Key | JSON type | Content |
|---|---|---|
| `id` | int | slot number ([Slot files](#slot-files)) |
| `pictureData` | string | base64 PNG thumbnail, 320 × 180; 1 086 to 77 375 bytes of PNG measured |
| `userData` | string, JSON | party, characters, inventory, counters ([Domain sections](#domain-sections)) |
| `configData` | string, JSON | in-game options of this slot ([Slot `configData`](#slot-configdata)) |
| `dataStorage` | string, JSON | progress flags ([Time stamp and progress flags](#time-stamp-and-progress-flags)) |
| `mapData` | string, JSON | position and map state ([Party position and map](#party-position-and-map)) |
| `timeStamp` | string | save date, layout set by the game ([Time stamp and progress flags](#time-stamp-and-progress-flags)) |
| `playTime` | float | `0.0` in all 14 game slots; the play time sits in `userData.playTime` |
| `clearFlag` | int | `1` in FF I slot 1, `0` in every other game slot |
| `isCompleteFlag` | int | equal to `clearFlag` except in FF I's Auto Save (`0` and `1`); meaning open |

### Telling the games apart

A slot file names no game.
The entry count of `userData → ownedTransportationList → target` identifies it: 4 for FF I, 7 for FF II, 14 for FF III, 11 for FF IV, 19 for FF V, 8 for FF VI [M, 16 slot files].
The count is the same in every slot file of a game (slot 1, Auto Save, Quick Save, slot 2) and takes six distinct values [M].
The game fills the list from its transportation table, one entry per vehicle, owned or not [I; D for FF I].
[Transportation](#transportation) holds the entries.

No explicit identifier exists [M]:

- Every slot file holds the same ten top-level keys in the same order ([Slot keys](#slot-keys)).
- No key path, at any depth, has a name containing `game`, `title`, `version`, `app`, `product`, `series`, `build` or `platform`.
  The hits on `type` or `game` are `configData.buttonType`, `mapData.viewType` and `userData.isOpenedGameBoosterWindow`, and none names the game.
- Of the 66 scalar leaves outside arrays that all 16 files hold, 8 keep one value per game and none takes six distinct values.

Key names separate three games out of six [M].
These paths need no array element to exist:

| Path | Game that holds it |
|---|---|
| `userData → cheatSettingsData.jobProficiencyRate` | FF III |
| `userData → winCount`, `braveBladeReliefCount`, `braveBladeEscapeCount`, `isTakeOverBraveBladeEscapeCount`, `wonderWandIndex` | FF V |
| `mapData → otherPartyDataList`, `partyPlayableCharacterCorpsId`, `currentSelectedPartyId`, `fieldDefenseNpcEntityIDList`, `beastFieldEncountExchangeFlags`, `beastFieldEncountSeekGroupId`, `rtsData` | FF VI |

Each path is present in every slot file of its game and absent from every other game's files [M].
FF I, FF II and FF IV share one key set, so no key without an array step tells them apart.
Other per-game paths (FF I `slotInfo` values, FF II `ownedKeyWaordList` elements, FF VI `currentConditionList` elements) exist only once an array holds an element, so they depend on progress and cannot serve as markers [I].

A format applies this rule when it opens a slot file for game G:

1. Refuse when `userData` is absent or holds no JSON object.
   The bestiary, slot index and system settings files fail here.
2. Read the `target` array of `userData → ownedTransportationList`.
   Refuse when the list or `target` is missing or `target` is not an array.
3. Map the length of `target` to a game with the counts above.
   Refuse when the length maps to no game or to a game other than G, and name the game that owns the count, with the count, in the error.
   Step 3 runs before step 4: a file with the FF II count and an FF VI marker reports FF II.
4. Refuse when a marker contradicts the count: `userData → cheatSettingsData.jobProficiencyRate` present unless G is FF III, `userData → wonderWandIndex` present unless G is FF V, `mapData → otherPartyDataList` present unless G is FF VI.
   The error names the marker's game and the count.
   A marker path that crosses a value of the wrong kind, such as `cheatSettingsData` holding a number, is no absent key: NeoFFSE refuses the file as not a slot [I].

Step 4 only refuses.
A game's own markers are never required, because the count decides and a build could drop a key.
The rule keys on the count, not on the full ID set, so a list the game reorders still passes.

The FF I format applies the rule; NeoFFSE has no format for FF II to VI yet.
The saves give the FF II to VI counts [M], while the tables behind them stay unread, so each count remains a reading of the saves ([Open questions](#open-questions), 16 to 19).

After the game rule, the FF I format checks the shape of what it edits [I].
`ownedCharacterList`, `normalOwnedItemList` and `importantOwendItemList` must exist, each row key (`id` of a character, `contentId` of an item) must be an unsigned 32-bit integer, and no key repeats inside a list ([Characters](#characters), [Inventory](#inventory)).
The format refuses a file that fails a check as not a slot.

### Documents inside strings

These keys hold a JSON document serialized as a string [M]:

```text
userData
userData.corpsList, .corpsSlots, .ownedCharacterList, .releasedJobs
userData.normalOwnedItemList, .importantOwendItemList, .normalOwnedItemSortIdList
userData.ownedTransportationList, .owendCrystalFlags, .warehouseItemList
userData.ownedKeyWaordList, .ownedMagicList, .learnedAbilityList, .ownedMagicStoneList
  each element of most "target" and "values" arrays (see Container shapes)
character: parameter, commandList, abilityList, abilitySlotDataList, jobList,
  equipmentList, additionOrderOwnedAbilityIds, sortOrderOwnedAbilityIds,
  abilityDictionary, skillLevelTargets, learningAbilitys, equipmentAbilitys
parameter: currentMpCountList, addtionalMaxMpCountList, currentConditionList
abilitySlotDataList element: slotInfo
configData, dataStorage, mapData
mapData: playerEntity, gpsData
bestiary: scenarioFlags, monsterDefeats
system settings: gamePadKeySettings, keyboardKeySettings, mouseKeySettings
```

The same keys hold documents in all six games, and every string document is an object, never a bare array [M].

- Each string level doubles the escaping: a quote at depth n appears as `2^n − 1` backslashes and a quote [M, 34 files].
  FF I spell slots reach depth 7 (127 backslashes); FF II to VI reach depth 6 (63 backslashes) [M].
- `userData.cheatSettingsData` is a plain nested object, not a string [M].
- An absent document is an empty string `""`, never `"{}"` or `"[]"`: see `companionEntity`, `telepoCacheData`, `timerData`, `userData.configData`, empty spell slots, `mapData.rtsData` and, in FF VI, `playerEntity` in each `otherPartyDataList` entry [M].
  Keep `""` as `""`.
  `rtsData` and the FF VI `playerEntity` occur only as `""` in the measured files; their filled form is [C].

### Container shapes

The game builds documents from two shapes [M]:

| Shape | JSON | Notes |
|---|---|---|
| List | `{"target":[…]}` | elements are ints, booleans or JSON strings |
| Dictionary | `{"keys":[…],"values":[…]}` | parallel arrays of equal length; values are ints or JSON strings |

An object stored in a list is a JSON string, so a list of items reads `{"target":["{\"contentId\":62,\"count\":5}", …]}` [M].

### Numbers

- Floats keep a decimal point: `58.0`, `1.0`, `0.0` [M].
  In slot 1 of FF III to VI, every float but `userData.playTime` is integral and keeps `.0`.
  Go's JSON encoder writes `58` and `1`, so the Go editor rewrites each float it re-serializes as an integer [M, measured on `go-ordered-json` and two editor files].
- No number uses an exponent or a leading zero [M].
- The game writes doubles with up to 17 significant digits, not always in the shortest round-trip form [M].
  Of the non-integral `userData.playTime` values, 4 carry 17 digits whose last digit differs by 1 or 2 from the shortest form of the same double; both texts parse to the same double [M].
  Invented values show the property: `13621.814333377138` and `13621.814333377139` parse to one double, and the second is its shortest form.
  A serializer that prints a parsed double rewrites those 4 values.
  The correctly rounded 17-digit form equals the shortest form, so the game's last digits are not correctly rounded [M].
- Integers exceed the i32 range.
  `dataStorage.treasure` holds `4294967295` and `dataStorage.scenario` `4293655417` (u32 values), and `timeStampTicks` reaches 18 digits [M].
  An invented value such as `639083329294237651` is above 2^53, so a parser that maps each number to an f64 (JavaScript's `JSON.parse`, for one) rounds it.
  FF III and FF IV saves hold no integer above i32.
  FF V and FF VI each hold a `timeStampTicks` above 2^53, and FF VI a `scenario` above i32 [M].
- Keep the original text of each number you do not edit ([Principles](../architecture.md#principles), principle 1).
- Rust's `serde_json` 1.0.154 with `preserve_order` and `arbitrary_precision` re-serializes all 5 527 JSON documents of the 34 files, at every nesting depth from 0 to 7, byte for byte, and re-escapes all 7 392 string tokens byte for byte [M].
  Without `arbitrary_precision`, 4 `userData` documents break on `userData.playTime`; without `preserve_order` as well, 1 266 documents break on key order [M].

### Key order and spellings

Keep the key order you read.
Keys shared by the six games appear in one relative order, and no object repeats a key [M].
The game's misspellings are the real key names [M]:

| Key as written | Location |
|---|---|
| `owendGil` | `userData` |
| `importantOwendItemList` | `userData` |
| `owendCrystalFlags` | `userData` |
| `ownedKeyWaordList` | `userData` |
| `learningAbilitys`, `equipmentAbilitys` | character |
| `numberOfButtles` | character |
| `addtionalLevel` to `addtionalMagicDefenseCount` (24 keys) | `parameter` |
| `addtionalMaxMpCountList` | `parameter` |
| `addionalWeight` | `parameter` |
| `isEnableEncount` | `userData.cheatSettingsData` |
| `encountEnable` | `mapData` |
| `resolutionHight` | system settings |

### Keys that differ per game

`mapData.minimapEnable` exists in all six FF I and FF II slots [M].
FF III to VI saves hold keys that FF I and FF II lack, per game [M].
A re-serializer that rebuilds each document from a fixed set of fields drops them ([Known pitfalls](#known-pitfalls), 7).

| Document | Extra keys | Games |
|---|---|---|
| `mapData` | `viewType` | III, V, VI |
| `mapData` | `currentSelectedPartyId`, `otherPartyDataList`, `partyPlayableCharacterCorpsId`, `fieldDefenseNpcEntityIDList`, `beastFieldEncountExchangeFlags`, `beastFieldEncountSeekGroupId`, `rtsData` | VI |
| `gpsData` | `transportationId` | III, IV, VI |
| `userData` | `wonderWandIndex` | V |
| `userData` | `winCount`, `braveBladeReliefCount`, `braveBladeEscapeCount`, `isTakeOverBraveBladeEscapeCount` | V |
| `cheatSettingsData` | `jobProficiencyRate` | III |

No measured slot holds `configData.brightness`, `masterVolume`, `bgmVolume` or `seVolume`, which the Go structs declare [M, absence].

### Documents of the other files

| File | Keys, in order [M] |
|---|---|
| Bestiary | `scenarioFlags` (string, JSON), `monsterDefeats` (string, JSON), `totalSubjugationCount` (int) |
| Slot index | `keys` (ints 1 to 22), `values` (booleans) |
| System settings | `language`, `fontType`, `gamePadKeySettings`, `keyboardKeySettings`, `mouseKeySettings`, `screenMode`, `resolutionWidth`, `resolutionHight`, `isLeftActionIcon`, `isLeftVirtualPad`, `isLeftMenuCommand`, `isLeftBattleCommand`, `isAnalogMode`, `brightness`, `masterVolume`, `bgmVolume`, `seVolume`, `bgmType` |

The three `…KeySettings` values are dictionaries whose values are more dictionaries in strings [M].

## Domain sections

### Game master data

The game stores its IDs and their names in its own data files.
`content.csv` gives each `contentId` a type and a name key, and the message files turn the key into text in each language.
NeoFFSE builds its tables from these files.
Every fact below and in the four sections after it comes from the FF I install, build 20249397 [D].
The installs of the other five games were not read.

**Where the files sit.**
`<install>` is `<library>/steamapps/common/FINAL FANTASY PR/`.
Both files are TextAssets inside Unity Addressables bundles in `<install>/FINAL FANTASY_Data/StreamingAssets/aa/StandaloneWindows64/`:

| Bundle | TextAsset | Holds |
|---|---|---|
| `master_assets_all_<hash>.bundle` | `Assets/GameAssets/Serial/Data/Master/*.csv` | 89 CSV files; `content.csv` defines the content IDs |
| `message_assets_all_<hash>.bundle` | `Assets/GameAssets/Serial/Data/Message/system_en.txt`, `system_fr.txt`, `system_ja.txt` | text per language, one file each |

The `<hash>` part of the bundle names changes with the build.
Match the exact prefixes `master_assets_all_` and `message_assets_all_`: a looser match on `message_assets_all_` hits `common_message_assets_all_<hash>.bundle` too, which holds UI sprites.
`StreamingAssets/aa/catalog.json` (Addressables 1.16.10) gives another way in: its `m_InternalIds` lists every bundle path with its hash, and the asset paths inside the bundles.
Those asset paths, such as `Assets/GameAssets/Serial/Data/Master/content.csv`, carry no hash [D].
UnityPy 1.25.4 opens the bundles [D].

**`content.csv`.**
One row per content ID, with a header line.
The measured build holds 221 rows [D].
Columns, in order:

| Column | Holds |
|---|---|
| `id` | the `contentId` a save stores |
| `mes_id_name` | key of the name in the message files |
| `mes_id_battle` | a second message key; `None` on the rows quoted below |
| `mes_id_description` | key of the description in the message files |
| `icon_id` | an icon number; `0` on the rows quoted below |
| `type_id` | the kind of row, below |
| `type_value` | the row ID in the master file of that kind |

| `type_id` | Kind | `type_value` points to | Rows |
|---|---|---|---|
| 1 | item | row in `item.csv` | 36 |
| 2 | weapon | row in `weapon.csv` | 41 |
| 3 | armor | row in `armor.csv` | 44 |
| 4 | ability | row in `ability.csv` | 99 |
| 5 | gil | not measured | 1 |

Type 4 holds the names of the abilities ([Ability, job and command tables](#ability-job-and-command-tables)).
Types 1 to 3 hold 121 content IDs, from 2 to 202 [D].
The `id` column is the `contentId` of saves: 111 entries of the Go editor's tables, which use save IDs, match it by ID and name ([Go editor tables](#go-editor-tables)) [D, M].

**Resolving a name.**
Look up `mes_id_name` in the message file of the language.
A message file holds one `key<TAB>text` pair per CRLF line, and no key repeats [D].
`system_en.txt` and `system_fr.txt` hold the same 1 099 keys.
Every name key of the tables in this document resolves in them, and in no other message file (`story_cha`, `story_mes` and `etc_text` do not hold them) [D].
The game resolves names this way [I]: the column name and the matches against the Go editor's tables say so.

- **Icon tags.**
  A name can start with an icon tag matching `^<IC_[A-Z]+>`: `<IC_SHIELD>` on shields, `<IC_WMGC>` and `<IC_BMGC>` on spells.
  The files hold 19 distinct tags.
  Strip the tag before you show the name [D].
- **Plain text.**
  After the tag, a name holds no `"`, `\`, tab, `<` or `>`, and no space at either end.
  English names are ASCII.
  French names add `É â è é ê ë î ï ô û Œ œ` [D].
- **Descriptions.**
  A description holds a literal `\n` (a backslash and an `n`), not a line break [D].

A name key does not follow the content ID, so read the key from the row.
Rows 164 to 166, verbatim [D]:

```text
164,MSG_ARMOR_NAME_21,None,MSG_ARMOR_INF_21,0,3,35
165,MSG_ARMOR_NAME_23,None,MSG_ARMOR_INF_23,0,3,36
166,MSG_ARMOR_NAME_22,None,MSG_ARMOR_INF_22,0,3,37
```

| Content ID | Name key | English | French | `armor.csv` row |
|---|---|---|---|---|
| 165 | `MSG_ARMOR_NAME_23` | Ice Shield | Bouclier gelé | 36 |
| 166 | `MSG_ARMOR_NAME_22` | Flame Shield | Bouclier igné | 37 |

The rows, not the keys, carry the right name [D]:

- The description under `MSG_ARMOR_INF_23` (on 165) describes an ice shield that guards against fire, and `MSG_ARMOR_INF_22` (on 166) a fire shield that guards against ice.
  The English, French and Japanese files agree.
- `armor.csv` rows 36 and 37 differ in `resistance_attribute` (117 and 118), which `attribute_group.csv` maps to attribute 1 and attribute 2.
  Fire and Ice sit at consecutive message keys (`MSG_SYSTEM_240`, `MSG_SYSTEM_241`), so 1 is fire and 2 is ice [I].
  The shield on 165 resists fire, which fits Ice Shield.
- `sort_id` is 45 on row 36 and 46 on row 37: the game lists Ice Shield before Flame Shield.

**Other master files** read [D]:

- `armor.csv` holds one row per armor-type content ID (44 rows).
  Its columns include `parts_group_id`, `resistance_attribute` and `sort_id`, and one each for defense and price.
  `parts_group_id` is the equipment slot key of the row [D]:

  | `parts_group_id` | Slot | Rows | Empty-slot row |
  |---|---|---|---|
  | 1 | right hand | 41 `weapon.csv` rows, 62 included | content 62 |
  | 2 | left hand, shields | 10 | content 130 (`armor.csv` row 1) |
  | 3 | body, armor | 17 | content 131 (row 2) |
  | 4 | head, helmets | 8 | content 132 (row 3) |
  | 5 | arms, gloves | 9 | content 133 (row 4) |

  The counts include the empty-slot row.
  Group 2 holds the nine shields (rows 32 to 37, 39, 41 and 44) beside the empty left hand.
  No row that `content.csv` reaches falls outside these groups.
- `attribute_group.csv` maps a `resistance_attribute` value to attributes.
- `item.csv` holds the rows that type 1 points to, and its `type_id` separates items from key items (below).
  `weapon.csv` holds the rows that type 2 points to; only its `parts_group_id` and `max_hp_up` were read.

**Item or key item.**
`content.csv` gives both kinds `type_id` 1.
The `type_id` of `item.csv` (36 rows) splits them, and a content row reaches its `item.csv` row through `type_value` [D]:

| `item.csv` `type_id` | Rows | Content IDs | Name keys | Icon tags |
|---|---|---|---|---|
| 1 | 19 | 2 to 36, with gaps | `MSG_ITEM_NAME_*` | `<IC_DRAG>`, `<IC_BAG>`, `<IC_TENTO>` |
| 2 | 17 | 45 to 61 | `MSG_KEY_NAME_*` | `<IC_IOBJ>` |

Example: `item.csv` row 1 (`1,1,1,1`: `id`, `sort_id`, `type_id`, `system_id`) is content 2, a Potion, and row 45 (`45,45,2,0`) is content 46, a Crown.
Every type-2 row has `system_id` 0, a sell price of 0 and both function groups at 0.
Every type-1 row has a menu or battle function group other than 0.
The labels "item" and "key item" come from the name-key prefixes and the icons, not from game code [I].

**Growth file.**
`data/pr/ff1/growth.toml` holds the growth tables of [Maximum HP and experience tables](#maximum-hp-and-experience-tables) in a compact shape.
Each `[[character_status]]` entry holds `id`, `hp` and `vitality`, and `[exp_table]` holds `total_exp` as one list in level order from level 1.
Each `[[intermediate_growth_curve]]` entry holds `job_id` and `growth_curve_group_id`.
Each `[[growth_curve]]` entry covers one growth group: `group_id`, `first_lv` and one list per column of `growth_curve.csv`, in level order from `first_lv`.

### Maximum HP and experience tables

The game computes the maximum HP and the experience to the next level from three master files.
The save stores neither figure, so a tool that shows them reads these tables, and every fact below is [D] unless a tag says otherwise.

**Maximum HP.**
`character_status.csv` (6 rows) holds the base HP in its `hp` column: 35, 30, 33, 30, 33 and 25 for `id` 1 to 6.
A save's `characterStatusId` selects the row; the row is the base class, and `intermediate_growth_curve.csv` maps jobs 1 and 7 to character 1, jobs 2 and 8 to character 2, and so on.
`game_constant_int.csv` holds `MAX_HP` 999 ([Bounds in the master data](#bounds-in-the-master-data)).
The `max_hp_up` column of `weapon.csv` (41 rows) and `armor.csv` (44 rows) holds 0 on every row, and the stat columns of `job.csv` hold 0 on all 12 rows, so equipment and job add no HP.
The formula is in [Characters](#characters).

**Experience.**
`exp_table.csv` (99 rows: `id`, `type_id`, `group_id`, `lv`, `total_exp`) gives the total experience a character needs to reach a level.
Level 2 holds 14 and level 99 holds 1 471 455, and each row's `total_exp` exceeds the row before.
Level 1 holds 0, so the figure is the total to reach a level and cannot be the total to leave it.
`id` equals `lv`, and every row holds 1 in `type_id` and in `group_id`.
FF I has one experience curve: `intermediate_growth_curve.csv` (12 rows, one per job 1 to 12) holds `exp_table_group_id` 1 on every row.

**Export requirement.**
A table export for these figures keeps `character_status.csv` (`id`, `hp`) and `exp_table.csv` (`lv`, `total_exp`).
It checks that `group_id` is 1 on every `exp_table.csv` row and `exp_table_group_id` is 1 on every `intermediate_growth_curve.csv` row.
A build that adds a second curve then fails the export instead of yielding wrong numbers [I].

**Next-level figure.**
Let L be the highest level with `total_exp(L) <= currentExp`.
The figure is `total_exp(L + 1) - currentExp`.
It is never negative, and the table has no row for level 100, so a character at L = 99 has no figure [I].
The game computes it from the experience alone and ignores the stored `addtionalLevel`, while the menu prints `addtionalLevel` as the level [B, G].

**Growth tables.**
A level-up reads two more tables ([Level-up in FF I](#level-up-in-ff-i)).
`intermediate_growth_curve.csv` (12 rows: `character_id`, `job_id`, `growth_curve_group_id`, `exp_table_group_id`) gives jobs 1 to 12 the growth groups 1 to 12.
The promoted jobs 7 to 12 have groups of their own [D].
`growth_curve.csv` (1 176 rows: 12 groups times levels 2 to 99) holds one row per group and level, the gain that reaching the level adds.
`character_status.csv` also holds the level-1 stats (`vitality` among them) and `magical_times1` to `magical_times8`, the level-1 spell charges [D].
Columns of `growth_curve.csv`, as the level-up code uses them [B]:

| Columns | Values in FF I [D] | Meaning |
|---|---|---|
| `insert_type_1` | 3 on every row | HP mode: 1 adds `hp_value1`, 2 adds a random value between `hp_value1` and `hp_value2`, 3 uses the formula in [Level-up in FF I](#level-up-in-ff-i) |
| `hp_value1` | 0 or 1 (210 rows hold 1) | in mode 3, 1 forces the strong HP bonus |
| `hp_value2`, `mp_value1`, `mp_value2` | 0 on every row | unused in FF I |
| `insert_type_2` | 3 on every row | stat mode: 1 default, 2 other tables, 3 formula |
| `strength`, `vitality`, `agility`, `intelligence`, `spirit`, `magic`, `luck` | 0 or 1; `spirit` and `magic` are 0 on every row | in mode 3, a non-zero value is added; a 0 becomes 1 on a 1-in-8 roll |
| `insert_type_3` | 1 on every row | mode 1 adds `accuracy_rate` and `magic_evasion_rate` as they stand, with no roll |
| `accuracy_rate`, `magic_evasion_rate` | constant per group (Fighter 3 and 2, Knight 4 and 3) | added to `addtionalAccuracyRate` and `addtionalAbilityEvasionRate` |
| `magical_times1` to `magical_times8` | Knight and Ninja groups 0 or 1; mage groups a running total from 0 to 9 | spell charges per spell level |

### Ability, job and command tables

Abilities, jobs and commands each have a master file whose name keys resolve in `system_en.txt` and `system_fr.txt`.
Abilities reach their names through `content.csv`; jobs and commands carry the key themselves.
Every fact in this section is [D] unless a tag says otherwise.

**Abilities.**
`ability.csv` holds 96 rows: IDs 1 to 95 and 100.
It has no name column.
The columns that matter here are `id`, `sort_id`, `ability_lv`, `ability_group_id`, `type_id`, `use_job_group_id` and `buy`.

The name of `abilityId` N sits in the `content.csv` row of type 4 whose `type_value` is N.
Join on `type_value`, not on a fixed offset:

- 98 of the 99 type-4 rows have a content ID equal to `type_value + 208`.
  The exception is content 310, which points to ability 100, an offset of 210.
  Ability 100 is a second Curaja (`type_id` 1, `ability_lv` 7, `use_job_group_id` 0).
- Each `ability.csv` row matches one content row and no other.
- Content 305, 306 and 307 (`type_value` 97, 98 and 99, all named "Attack") point to no `ability.csv` row.

`ability.csv` `type_id` sorts the 96 rows into kinds:

| `type_id` | IDs | Rows | Name keys | Icon tag |
|---|---|---|---|---|
| 1 | offsets 0 to 3 of each level in 5 to 68, and 100 | 33 | `MSG_MAGIC_NAME_*` | `<IC_WMGC>` |
| 2 | offsets 4 to 7 of each level in 5 to 68 | 32 | `MSG_MAGIC_NAME_*` | `<IC_BMGC>` |
| 13 | 69 to 94 except 77 and 91 | 24 | `MSG_MON_ATK_*` | none |
| 3 | 77 and 91 | 2 | `MSG_MON_ATK_*` | none |
| 4, 5, 6, 8 | 1 (type 4), 2 (type 5), 3 and 95 (type 6), 4 (type 8) | 5 | `MSG_SYSTEM_*` (Attack, Defend, Flee, Back) | none |

No table in the data names these kinds.
Type 1 as white magic and type 2 as black magic rest on the icon tags and on the white and black offsets measured in the saves [I].
Types 3 and 13 are both monster abilities by their name keys; the data does not say what separates them [I].

`ability_lv` holds the spell level, 1 to 8, and 0 for every non-spell.
For IDs 5 to 68, `ability_lv` = (`id` − 5) / 8 + 1 with integer division, and `type_id` is 1 for offsets 0 to 3 and 2 for offsets 4 to 7, without exception.
The formula measured in the saves ([Abilities](#abilities)) holds for all 64 spells.
`use_job_group_id` points into `job_group.csv` (50 rows, columns `job1_accept` to `job22_accept`), which says which jobs may learn the spell; its values stay undecoded.

**Jobs.**
`job.csv` holds 12 rows, IDs 1 to 12, with the columns `id`, `mes_id_name`, `mes_id_description`, `change_command_1` to `change_command_5`, `initial_condition`, `strength`, `vitality`, `agility` and `magic`.
The name keys are `MSG_JOB_NAME_01` to `MSG_JOB_NAME_12`, in ID order.
Job 1 reads Warrior (Guerrier), job 7 Knight (Paladin) and job 12 Black Wizard (Sage noir).
The promoted jobs carry no description key (`7,MSG_JOB_NAME_07,None`).
`change_command_1` to `change_command_5` are 0 on all 12 rows.

**Commands.**
`command.csv` holds 6 rows, IDs 1 to 6, with the columns `id`, `command_lv`, `mes_id_name`, `mes_id_name2`, `mes_id_description`, `ability_id` and others:

| ID | `mes_id_name` | English | French | `ability_id` |
|---|---|---|---|---|
| 1 | `MSG_SYSTEM_029` | Attack | Attaque | 1 |
| 2 | `MSG_SYSTEM_030` | Defend | Défense | 2 |
| 3 | `MSG_SYSTEM_043` | Magic | Magie | 0 |
| 4 | `MSG_SYSTEM_042` | Items | Objets | 0 |
| 5 | `MSG_SYSTEM_032` | Flee | Fuite | 3 |
| 6 | `MSG_SYSTEM_002` | Back | Retour | 4 |

`mes_id_name2` equals `mes_id_name` on all six rows.
A save's `commandList` `[1,3,4,2,5]` reads Attack, Magic, Items, Defend, Flee [M, D].
`character_status.csv` (6 rows) gives every starting character `command_id1` to `command_id3` of 1, 3 and 4.

### Map and monster tables

Maps and monsters each have a master file with name keys.
Map names need two joins, and both files repeat names, so key every table by ID.
Every fact in this section is [D] unless a tag says otherwise.

**Maps.**
`map.csv` holds 130 rows: IDs 1 to 127 without a gap, then 800, 801 and 802.
A row carries two name keys:

- `area_id` points into `area.csv` (29 rows: `id`, `map_type`, `area_name`).
  `area_name` is a `MSG_MAP_NAME_*` key for a place such as "Castle Cornelia" or "World Map".
- `map_title` is a `MSGARA_NAME_*` key for a floor or a shop ("1F", "Inn"), or `None`, which 39 rows hold.

`map_name` (`Map_20011_1`) and `asset_name` (`map_20011`) are internal labels.
Example: map 2 has `map_title` `MSGARA_NAME_06` and `area_id` 16, whose `area_name` is `MSG_MAP_NAME_08`.

Names repeat:

- Joined as area name plus title, the 130 rows give 101 distinct English names.
  17 names repeat: Castle Cornelia seven times, Gaia six times and World Map four times.
- Two `area.csv` rows both read "Sunken Shrine".
- `map_name` is unique per row, so it makes a stable internal label.
- Rows 800 to 802 (`Map_00000_*`) sit in area 1 with no title, so they read "World Map" too.
  The data does not say what they serve.
- French titles start with a lower-case letter ("rez-de-chaussée", "auberge").
  The data does not hold the format the game uses to join area name and title [I].

**Monsters.**
`monster.csv` holds 129 rows, IDs 1 to 129 without a gap, and `mes_id_name` is a `MSG_MON_NAME_*` key.
The rows carry 124 distinct names: five keys each serve two rows.

| Name | Monster IDs | `boss` | HP |
|---|---|---|---|
| Vampire | 61, 129 | 0, 1 | 280, 280 |
| Lich | 120, 121 | 1, 1 | 1 200, 2 800 |
| Marilith | 122, 123 | 1, 1 | 1 440, 3 200 |
| Kraken | 124, 125 | 1, 1 | 1 800, 3 600 |
| Tiamat | 126, 127 | 1, 1 | 2 400, 5 500 |

The column `boss` is 1 on 12 rows: 106, 114 and 120 to 129.
Monster 128 is Chaos.
The keys of the saved bestiary are these monster IDs ([Bestiary](#bestiary)).

**Export rule.**
`picture_book_group.csv` folds the two Vampires into one group, so a table with one monster per group holds 128 monsters and drops monster 129, the boss Vampire ([Bestiary](#bestiary)) [D].

### Bounds in the master data

The game states six caps and several stat caps in `game_constant_int.csv` (96 rows: `id`, `key_name`, `value1`, `value2`) [D]:

| Save field | `key_name` | `value1` |
|---|---|---|
| `owendGil` | `MAX_GIL` | 999 999 |
| `normalOwnedItemList` `count` | `MAX_ITEM` | 99 |
| level (`addtionalLevel`) | `MAX_LV` | 99 |
| `currentExp` | `MAX_EXP` | 9 999 999 |
| HP | `MAX_HP` | 999 |
| spell charges per level | `MAX_MAGICAL_TIMES` | 9 |
| strength, vitality, agility, intelligence, spirit, luck | `MAX_STRNGTH`, `MAX_VITALITY`, `MAX_AGILITY`, `MAX_INTELIGENCE`, `MAX_SPRIT`, `MAX_LUCK` | 99 each |
| attack, defense, ability attack, ability defense, accuracy, evasion | `MAX_ATTACK`, `MAX_DEFENSE`, `MAX_ABILITY_ATTACK`, `MAX_ABILITY_DEFENSE`, `MAX_ACCURACY_RATE`, `MAX_EVASION_RATE` | 255 each |

Nine more `MAX_` rows have no known save field [I]: `MAX_ACCURACY_RATE_PERCENTAGE`, `MAX_EVASION_RATE_PERCENTAGE`, `MAX_ABILITY_EVASION_RATE` (open: [question 26](#open-questions)), `MAX_ABILITY_EVASION_RATE_PERCENTAGE`, `MAX_CRITICAL_RATE`, `MAX_CRITICAL_RATE_PERCENTAGE`, `MAX_ATTACK_SPEED`, `MAX_HIT_VALUE` and `MAX_GETAWAY_PROBABILITY_RATE`.

`MAX_ITEM` is one row with no list name [D].
NeoFFSE reads it as the cap of the normal inventory and not of the key items, which hold `count` 1 ([Inventory](#inventory)) [I].

The key names keep the game's spelling.
`exp_table.csv` (99 rows: `id`, `type_id`, `group_id`, `lv`, `total_exp`) ends at level 99 with a `total_exp` of 1 471 455, which agrees with `MAX_LV`.

The master data does not state:

- an MP cap: it holds no `MAX_MP` row, and FF I uses spell charges;
- a cap on the magic stat: it holds no row for it;
- a cap on the gain fields `addtionalMaxHp` and `addtionalMaxMpCountList`.

`MAX_HP` caps the displayed sum of base and gain, yet the stored gain `addtionalMaxHp` reaches 1 097 in the saves ([Characters](#characters)) [M, G, I].
Whether the game clamps an edited gain field is open ([Open questions](#open-questions)).

**Caps in NeoFFSE.**
A cap is a limit the game states; a storage bound is the range a save field can hold (next paragraph).
Under `CapPolicy::Enforce` and in the game interface, NeoFFSE applies a cap to the stored field that carries the value [I]:

| Stored field | Cap |
|---|---|
| `owendGil` | `MAX_GIL` |
| `count` in `normalOwnedItemList` | `MAX_ITEM` |
| `addtionalLevel` | `MAX_LV` |
| `currentExp` | `MAX_EXP` |
| `currentHP` | `MAX_HP` |
| each value of `currentMpCountList` | `MAX_MAGICAL_TIMES` |
| `addtionalPower`, `addtionalVitality`, `addtionalAgility`, `addtionalIntelligence`, `addtionalSpirit`, `addtionalLuck` | `MAX_STRNGTH`, `MAX_VITALITY`, `MAX_AGILITY`, `MAX_INTELIGENCE`, `MAX_SPRIT`, `MAX_LUCK` (99 each) |

The six stat fields hold gains over a base the file does not store, and NeoFFSE applies the stat cap to the stored gain.
No stored gain of the four measured FF I slots exceeds 99 [M, 4 slots].
NeoFFSE caps `currentHP`, not the gain `addtionalMaxHp`: it reads `currentHP` as bounded by the displayed maximum that `MAX_HP` limits, while the game caps the sum of base and gain, so a stored gain above 999 stays legal.
The magic stat (`addtionalMagic`), `addtionalMaxHp`, `addtionalMaxMpCountList` and the equipment slot `count` have no cap: the master data states none.

**Storage bound.**
Neither the master data nor the saves state a bound for the integer fields NeoFFSE edits, apart from the caps above.
NeoFFSE stores a value from 0 to 2 147 483 647 (`i32::MAX`) in such a field, and from 1 in the `count` of an item row and in a row key (`contentId`, character `id`) [I].
The Level is the exception: its storage bound is `MAX_LV`, so both edit interfaces refuse a Level above 99 ([Characters](#characters)) [I].
The `count` of an equipment slot starts at 0: no measured slot holds 0 [M, 4 slots], and the game interface sets that count only by copying the inventory count [I].
The Go editor clamps its number fields to i32 ([Known pitfalls](#known-pitfalls), 10), and every counter measured in FF I fits.
No test shows that the game reads the whole range.
A value above a cap and inside the storage bound is storable.

Both edit interfaces refuse a value outside the storage bound [I].
The game interface refuses a key-item `count` other than 1 and any hand-set `count` of an equipment slot ([Inventory](#inventory), [Equipment slots](#equipment-slots)) [I].

### ID spaces and tables

A save points into the game data through six ID spaces: `contentId`, `abilityId`, `jobId`, commands, maps and monsters.
NeoFFSE takes the IDs, kinds and names of all six from the game's master files and message files ([Game master data](#game-master-data), [Ability, job and command tables](#ability-job-and-command-tables), [Map and monster tables](#map-and-monster-tables)).
Facts about game data are [D], facts about the Go editor's tables [C], comparisons with the saves [M].

| Space | Referenced by | Names from | IDs |
|---|---|---|---|
| `contentId` | rows of `normalOwnedItemList` and `importantOwendItemList`, equipment slots, the `contentId` of an ability object | `content.csv` with `system_en.txt` and `system_fr.txt` [D] | 2 to 202; spells 213 to 276 |
| `abilityId` | `abilityList`, spell slots, `learnedAbilityList` | the type-4 row of `content.csv` whose `type_value` is the ID [D] | 1 to 95 and 100 in the game [D]; 5 to 68 in the saves [M] |
| `jobId` | character `jobId`, `id` in `jobList`, `releasedJobs` | `job.csv` [D] | 1 to 12 |
| command | `commandList` | `command.csv` [D] | 1 to 6 in the game [D]; 1 to 5 in the saves [M] |
| map | `mapData.mapId` [C] | `map.csv`, through `area.csv` and `map_title` [D] | 1 to 127 and 800 to 802 |
| monster | `monsterDefeats` keys | `monster.csv` [D] | 1 to 129 in the game [D]; 1 to 128 in the saves [M] |

Items, key items and equipment share one `contentId` space.
Key items are the content IDs 45 to 61, the rows of type 2 in `item.csv` [D], and every ID of `importantOwendItemList` falls in that range [M].
Spell content IDs (`abilityId + 208` in FF I, with one exception, [Abilities](#abilities)) sit above every item and equipment ID, and none appears in the inventory [M].

- **Placeholders.**
  The game holds a row for each empty-slot ID: the name of 62 reads "Empty" (key `MSG_SYSTEM_069`) and the names of 130 to 133 read "None" (key `MSG_SYSTEM_070`) [D].
  They are content IDs the game uses for an empty slot, not items ([Equipment slots](#equipment-slots)).

### Go editor tables

The Go editor lists names for the content, ability, job, command, map and monster IDs in its package `models/core/costs/ff1`.
The game's files are the reference for all of them.
For `contentId`, 111 of its 116 entries match the game; for `abilityId`, 86 of 90; the other differences are known pitfalls ([Known pitfalls](#known-pitfalls), 28 and 29).
Each table is a package-level slice of `models.NameValue` (`Name string`, `Value int`), sorted by ID.

| Go table | Entries | IDs | Serves |
|---|---|---|---|
| `Items` | 18 | 2 to 36 | consumables in `normalOwnedItemList` |
| `ImportantItems` | 17 | 45 to 61 | `importantOwendItemList` |
| `Weapons` | 38 | 62 to 123 | equipment key 1, inventory |
| `Shields` | 10 | 130 to 173 | equipment key 2, inventory |
| `Armors` | 16 | 131 to 160 | equipment key 3, inventory |
| `Helmets` | 8 | 132 to 188 | equipment key 4, inventory |
| `Gloves` | 9 | 133 to 202 | equipment key 5, inventory |
| `Abilities` | 90 | 5 to 94 | `abilityId` |
| `Jobs` | 12 | 1 to 12 | `jobId` |
| `Commands` | 4 | 1 to 4 | `commandList` |
| `Maps` | 120 | 1 to 120 | `mapData.mapId` |
| `Bestiary` | 129 | 1 to 129 | `monsterDefeats` keys |

- **Content tables.**
  The seven content tables hold 116 entries over 115 distinct IDs.
  The only repeat, 165, sits inside `Shields`.
  The editor merges `Items` and the five equipment tables into one ID-to-name map, and the later entry replaces the earlier one [C].
- **Names.**
  All names are English ASCII, without `"` or `\`.
  Names repeat: "Gaze" six times, "Blaze" five times and "Icestorm" twice in `Abilities`, place names in `Maps` and boss variants in `Bestiary`.
  Look an entry up by ID, never by name.
- **Placeholders.**
  The first row of each equipment table is a placeholder: `[Empty R. Hand]` (62), `[Empty L. Hand]` (130), `[Empty Body]` (131), `[Empty Head]` (132) and `[Empty Arms]` (133).
  The labels are the editor's; the game names these rows "Empty" and "None".
- **Spells and monster abilities.**
  In `Abilities`, 5 to 68 are 64 spells, 8 levels of 8, and 69 to 94 are monster abilities that no character holds.
  In the Go table the names alone mark the split; the game's `ability.csv` marks it with `type_id` ([Ability, job and command tables](#ability-job-and-command-tables)).

**Against the game.**
The comparison covers the uncommented entries of the seven content tables at tag `v1.5.1`; the package is identical at `cea371f`.
An entry agrees when its ID exists in `content.csv` and its name equals the English name without the icon tag.
A placeholder entry agrees when the game row is a placeholder [D].

111 entries agree: `Items` 18, `ImportantItems` 17, `Weapons` 34, `Shields` 9, `Armors` 16, `Helmets` 8 and `Gloves` 9, with the five placeholders.
Five disagree:

| ID | Go table | Editor name | Game name at that ID | Cause |
|---|---|---|---|---|
| 76 | `Weapons` | Wyrmkiller Sword | Wyrmkiller | label only |
| 101 | `Weapons` | Sasuke's Blade | no such content ID | Sasuke's Blade is 100 |
| 102 | `Weapons` | Masamune | no such content ID | Masamune is 103 |
| 122 | `Weapons` | Wizard's Staff | Crosier | Wizard's Staff is 123; the editor names both 122 and 123 Wizard's Staff |
| 165 (second entry) | `Shields` | Flame Shield | Ice Shield | Flame Shield is 166 |

The game's types 1 to 3 hold eight content IDs that no Go content table lists [D].
Five of them appear in saves [M]:

| Content ID | Kind | Game name | Row |
|---|---|---|---|
| 67 | weapon | Cat Claws | `weapon.csv` 6 |
| 100 | weapon | Sasuke's Blade | `weapon.csv` 39 |
| 103 | weapon | Masamune | `weapon.csv` 42 |
| 151 | armor | Knight's Armor | `armor.csv` 22 |
| 166 | armor, shield | Flame Shield | `armor.csv` 37 |

The other three are 4 (X-Potion), 104 (Nunchaku) and 105 (Iron Nunchaku).
Apart from these eight, every gap inside the ranges of the Go tables is a gap in the game's IDs too [D].

**Abilities, commands, jobs and monsters against the game** [D].
The comparison reads `Abilities`, `Jobs`, `Commands` and `Bestiary` at tag `v1.5.1` against `ability.csv`, `job.csv`, `command.csv` and `monster.csv` with the message files.

`Abilities`: 86 of 90 entries agree.
The other four are two swaps.
The game numbers its name keys out of order, as it does for the shields, and its descriptions follow the rows:

| `abilityId` | Content row | Name key | Game name | Go name |
|---|---|---|---|---|
| 13 | 221 | `MSG_MAGIC_NAME_08` | Invis (raises an ally's evasion) | Blindna |
| 15 | 223 | `MSG_MAGIC_NAME_05` | Blindna (cures darkness) | Invis |
| 66 | 274 | `MSG_MAGIC_NAME_63` | Warp (banishes enemies to another dimension) | Stop |
| 67 | 275 | `MSG_MAGIC_NAME_62` | Stop (stops time and paralyzes all enemies) | Warp |

The other tables:

- `Commands`: the editor names 3 "Flee" and 4 "Row".
  The game names 3 Magic and 4 Items, puts Flee at 5 and Back at 6, and has no "Row".
  The editor's table stops at 4.
- `Jobs`: 12 of 12 names agree.
- `Bestiary`: 128 of 129 IDs agree.
  ID 129 reads "Vampire (Cavern of Earth)" in the editor and "Vampire" in the game.
- `Maps`: the editor's table ends at 120.
  The game's `map.csv` runs to 127 and adds 800 to 802.

### Characters

`userData → ownedCharacterList → target[]` holds one JSON string per character: 4 in FF I, 7 or 8 in FF II [M].
Identify a character by `id`; names repeat and change.

| Key | Type | Notes |
|---|---|---|
| `id` | int | 1 to N, unique [M]; NeoFFSE refuses a file that repeats an `id` [I] |
| `characterStatusId` | int | FF I 1 to 6, FF II 1 to 10 [M]. FF I: `jobId` = `characterStatusId` + 6 for 4 of 4 promoted characters [M]; the id names the `character_status.csv` row, the base class, whose `hp` is the base of the maximum HP ([Maximum HP and experience tables](#maximum-hp-and-experience-tables)) [M, D, G] |
| `isEnableCorps` | bool | 4 of 8 FF II characters are `false` [M]; the Go editor runs its per-game ability rules only for `true` [C] |
| `jobId` | int | FF I 7 to 12, FF II 1 to 9 [M]; FF I measured 7, 8, 11 and 12, all promoted classes; names in `job.csv` ([ID spaces and tables](#id-spaces-and-tables)) |
| `name` | string | raw UTF-8, may hold non-ASCII [M]; two FF I heroes may share a name, FF VI has two Mogs [C, `d8a6c6f`] |
| `currentExp` | int | [M]; the game caps it at 9 999 999 (`MAX_EXP`) [D] ([Bounds in the master data](#bounds-in-the-master-data)) |
| `parameter` | JSON | stats, below |
| `commandList` | JSON list of ints | FF I `[1,3,4,2,5]`, FF II `[1,3,4,2,5,7]` [M]; the Go editor writes it only when the user enabled command editing [C]. The FF I values read Attack, Magic, Items, Defend and Flee in `command.csv` [M, D] |
| `abilityList`, `abilitySlotDataList`, `additionOrderOwnedAbilityIds`, `sortOrderOwnedAbilityIds`, `abilityDictionary`, `skillLevelTargets`, `learningAbilitys` | JSON | [Abilities](#abilities) |
| `jobList` | JSON list of `{id, level, currentProficiency}` | FF I 12 entries (`id` 1 to 12), FF II 8 [M]. FF I: `level` 1 and `currentProficiency` 0 in every entry of the 8 character states [M]; FF I does not use them [I] |
| `equipmentList` | JSON dictionary | keys 1 to 5 (equipment slots), values `{contentId, count}` [M]; `count` above 1 occurs (FF I up to 7, FF II up to 4); slots and `count` in [Equipment slots](#equipment-slots) |
| `equipmentAbilitys` | JSON list | `[0,0,0]` in FF I and FF II [M] |
| `numberOfButtles`, `ownedMonsterId`, `magicStoneId`, `magicLearningValue` | int | `0` in FF I [M]; `magicStoneId` is the FF VI equipped esper [C] |

NeoFFSE edits the characters in place and never inserts or removes a character row [I].

`parameter` keys [M]: `currentHP`, `currentMP`, `currentMpCountList`, `addtionalMaxMpCountList` (dictionaries with 8 keys, one per spell level, in FF I and FF II), `addtionalLevel`, `addtionalMaxHp`, `addtionalMaxMp`, `addtionalPower`, `addtionalVitality`, `addtionalAgility`, `addionalWeight`, `addtionalIntelligence`, `addtionalSpirit`, `addtionalAttack`, `addtionalDefense`, `addtionalAbilityDefense`, `addtionalAbilityEvasionRate`, `addtionalMagic`, `addtionalLuck`, `addtionalAccuracyRate`, `addtionalEvasionRate`, `addtionalAbilityDisturbedRate`, `addtionalCriticalRate`, `addtionalDamageDirmeter`, `addtionalAbilityDefenseRate`, `addtionalAccuracyCount`, `addtionalEvasionCount`, `addtionalDefenseCount`, `addtionalMagicDefenseCount`, `currentConditionList` (`{"target":[]}` in all 35 measured characters).

**FF I `parameter` fields.**
Several `addtional…` names hold a gain on top of a base the file does not store:

- `addtionalLevel` grows with `currentExp` in both game states [M].
  In all 8 character states it equals the level the experience gives: `total_exp(addtionalLevel) <= currentExp < total_exp(addtionalLevel + 1)` ([Maximum HP and experience tables](#maximum-hp-and-experience-tables)) [M, D].
  The menu level equals the field, with no offset: the menu shows 69 for a value of 69 [G].
  The menu prints the stored field, not a level computed from the experience [B].
  With `addtionalLevel` 69 and `currentExp` at `total_exp(72)`, 884 216, the menu still shows level 69 after the load [G, edited copy].
  The Go editor labels it "Level" (`coreStats.go`) [C].
  The game caps the level at 99 (`MAX_LV`) [D].
- `currentExp` is the total experience, not the experience inside the level.
  The menu's "next level" figure comes from the experience alone: `total_exp(L + 1) - currentExp`, with L the level the experience gives ([Maximum HP and experience tables](#maximum-hp-and-experience-tables)).
  It matches the menu for all 4 characters of the matching game state: 20 295, 916, 11 681 and 11 681 [M, D, G].
  In those states L equals `addtionalLevel`, so a formula built on the stored field gives the same numbers.
  The edited copy above separates the two: with the field at 69 and L at 72, the menu showed 21 750, which is `total_exp(73) - currentExp` [G, edited copy].
- `addtionalMaxHp` is a gain over a per-class base, not the maximum.
  The file stores no maximum.
  The menu shows `min(base + addtionalMaxHp, MAX_HP)`, where `base` is the `hp` column of the `character_status.csv` row whose `id` equals the character's `characterStatusId` (35, 30, 33, 30, 33 and 25 for ids 1 to 6), and `MAX_HP` is 999 ([Maximum HP and experience tables](#maximum-hp-and-experience-tables)) [M, D, G].
  Equipment adds nothing: `max_hp_up` is 0 on every `weapon.csv` and `armor.csv` row [D].
  The formula gives the menu's maximum HP for all 4 characters of the matching game state: 999, 863, 930 and 679 [M, D, G].
  Character 1 exercises the cap: base plus gain is 1 132 and the menu shows 999.
  The stored gain exceeds the cap, and the game states no bound for the gain field [D].
  Whether the game clamps an edited gain field is open ([Open questions](#open-questions)).
  `currentHP` minus `addtionalMaxHp` equals the base when the character is at full HP, which holds for all 8 measured states.
  For a wounded character the difference falls below the base, so it cannot serve as the base.
  The Go editor labels the field "HP Max" (`coreStats.go`) [C], which the data contradicts.
- `addtionalSpirit` and `addtionalMagic` grow only through 1-in-8 rolls, since no table row carries a flag for them [M, D, B].
  The measured values are 6 to 9 for spirit and 7 to 12 for magic.
- `addtionalAbilityEvasionRate` holds the growth of the magic defence, and `addtionalAccuracyRate` the growth of the accuracy [M, D, B].
  Both follow the table sums exactly ([Level-up in FF I](#level-up-in-ff-i)).
  NeoFFSE edits them as the fields `magic_evasion_gain` and `accuracy_gain`, held by the storage bounds alone because no cap is known for them [I].
- `currentMP` and `addtionalMaxMp` hold 0 in all 8 states.
  FF I stores spell charges per spell level in `currentMpCountList` and `addtionalMaxMpCountList` (keys 1 to 8, values 0 to 9) [M].
  `addtionalMaxMpCountList` is a gain too: for the two spellcasters, level 1 shows `currentMpCountList` 9 against `addtionalMaxMpCountList` 7, and every other level shows equal values [M].
  Reading: maximum charges = base + `addtionalMaxMpCountList`, capped at 9 [I].
  The game states 9 charges per spell level as the cap (`MAX_MAGICAL_TIMES`) [D], and the menu shows a maximum of 9 charges at every spell level [G].
  The game states no MP cap, no cap on the magic stat and no cap on the gain fields ([Bounds in the master data](#bounds-in-the-master-data)) [D].

### Level-up in FF I

The file stores only the running gains, and the game adds one step per level gained at the end of a battle.
Part of each step is random, and the clock seeds the draw, so no tool can recompute the stored gains.
Every fact below is [B] unless a tag says otherwise.

**The rule.**
At the end of a battle that gives experience, the game adds the share to `currentExp`.
For each character that gets experience, it sets `addtionalLevel` to the level `exp_table.csv` gives for `currentExp`, higher or lower than the stored one [M, edited copy].
It then applies one step per level gained, from the old level plus 1 to the new level.
A character whose stored level lags its experience catches up at the next battle that gives it experience [M, edited copy].
Each step uses the `growth_curve.csv` row at that level in the growth group of the character's job at the end of the battle (`intermediate_growth_curve.csv`, [Maximum HP and experience tables](#maximum-hp-and-experience-tables)).
One step:

1. `addtionalMaxHp` gains `STA / 4 + 1` (integer division) [M, edited copy].
   `STA` is `character_status.vitality` plus `addtionalVitality`, taken before this step's own stat gains.
   NeoFFSE takes `character_status.vitality` as the base [I]; the exact HP gain of two characters, with bases 8 and 15, supports it (Evidence, below) [M, D].
   The game reads `STA` through `ConfirmedVitality`, which caps its result on read [B], and the cap is likely `MAX_VITALITY`, 99 [I].
   NeoFFSE applies no cap.
   The two differ once the sum passes 99, and the measured characters stay below it: 40 to 42 in the 8 states [M], 42 and 43 on the edited copy [M, edited copy].
   The step adds 20 to 24 more when `hp_value1` is 1 or a 1-in-8 roll succeeds.
2. Each of `addtionalPower`, `addtionalVitality`, `addtionalAgility`, `addtionalIntelligence`, `addtionalSpirit`, `addtionalMagic` and `addtionalLuck`, in that order, gains the value of its column [M, edited copy], or 1 on a 1-in-8 roll when the column holds 0.
3. `addtionalAccuracyRate` gains `accuracy_rate` and `addtionalAbilityEvasionRate` gains `magic_evasion_rate`, with no roll [M, edited copy].
4. `addtionalMaxMpCountList`: Knight (job 7) and Ninja (job 8) add `magical_timesN` to each level and cap every value at 4.
   Every other job replaces each value with `magical_timesN` of the row.
   The cap of 4 is a constant in the game code (`KnightMaxMagticCount` and `NinjaMaxMagicCount`); no master file holds it.

**Random parts.**
Every roll is a draw of 1 to 8 that succeeds on 1.
The HP roll always runs, so one draw is spent even when `hp_value1` is 1.
The game seeds the generator with the millisecond of the clock at the end of the battle (0 to 999), combined with the level before the battle and the character `id`, and reseeds before each step.
The save keeps no trace of the draw.

**What a step leaves alone.**
The game stores the raw sums and clamps nothing on write.
The menu caps what it shows (`MAX_HP`, the stat caps), which explains a stored `addtionalMaxHp` of 1 097 behind a displayed 999 [M, G].
A step leaves `currentHP`, `currentMpCountList` and `addtionalEvasionRate` unchanged.
The stored accuracy equals the table sum with no agility term, so the stat growth does not reach `addtionalAccuracyRate` [M].

**A lower experience level.**
When `currentExp` gives a lower level than the stored `addtionalLevel`, the game sets the lower level at the next battle that gives experience to the character [M, edited copy].
The step loop does not run, so no stat rises and none falls [M, edited copy].
A character at level 72 with `currentExp` at 800 000 (`total_exp(68)` 797 217, `total_exp(69)` 818 967) fell to level 68 after a battle that gave 7 experience.
No stat, gain or charge field moved, and `currentHP` changed by the battle damage alone.

**Evidence.**
The checks below run on the 8 character states of the two FF I game states ([Scope and sources](#scope-and-sources)).

- Accuracy and magic evasion match the table sums with one class change per character: levels 2 to 34 under the base group and 35 onward under the promoted group for two characters, 2 to 35 and 36 onward for the other two [M, D].
  Example, a character at level 69 with the change at 35: 3 × 33 + 4 × 35 = 239 for `addtionalAccuracyRate` and 2 × 33 + 3 × 35 = 171 for `addtionalAbilityEvasionRate`.
  Two characters fix the change level by two independent equations each, and the equations agree.
- The promoted groups 7 to 12 carry the same `hp_value1` and stat flags as the base groups 1 to 6, row by row (0 differences over 98 levels per pair).
  The job changes accuracy, magic evasion and spell charges only [D].
- In the 56 (character, stat, state) cases, no stored gain falls below the count of flagged levels.
  The 4 characters hold 196 points above their flags over 1 529 unflagged levels, 0.128 per level; a 1-in-8 rule predicts 191 ± 13 and a 1-in-4 rule about 382.
  `addtionalSpirit` and `addtionalMagic` grow with no flag at all, which only the roll on a zero column explains [M, D].
- Between the two game states, the HP gain per character equals the sum of `STA / 4 + 1` over the levels gained plus a whole number of bonuses of 20 to 24 [M, D]:

  | Levels gained | `STA / 4 + 1` sum | Strong levels flagged | HP gain stored | Bonuses that fit |
  |---|---|---|---|---|
  | 60 to 69 | 110 | 2 | 177 | 3 (2 flagged, 1 rolled) |
  | 60 to 68 | 48 | 1 | 93 | 2 (1 flagged, 1 rolled) |
  | 61 to 69 | 63 | 0 | 63 | none |
  | 61 to 69 | 45 to 53 | 1 | 92 | 2 |

  The third row shows the `+ 1` and the base vitality: without the base of 8, `STA` would be 16 to 19 and the sum 45 instead of 63.
  The first row rules out a fixed bonus of 24: three bonuses of 24 would give 72, and the rest holds 67.
- Catch-up on an edited copy: a Knight at `addtionalLevel` 69 with `currentExp` at `total_exp(72)` gained 63 experience in one battle.
  After the save, `addtionalLevel` read 72 and the fields below had moved by the sum of the three steps of levels 70 to 72 [M, edited copy]:

  | Field | Change | Why |
  |---|---|---|
  | `addtionalAccuracyRate` | +12 | 3 × `accuracy_rate` 4 |
  | `addtionalAbilityEvasionRate` | +9 | 3 × `magic_evasion_rate` 3 |
  | `addtionalPower`, `addtionalVitality`, `addtionalIntelligence`, `addtionalLuck` | +2, +1, +1 and +1 | the flagged levels among 70 to 72 in the job's growth group |
  | `addtionalMaxHp` | +33 | 3 steps of `STA / 4 + 1` = 11, with `STA` 42, 42 and 43 (base vitality 15), no bonus |

  No other stat field moved, and the Knight's spell charges stayed at the cap of 4.
  The other characters gained 63 experience and nothing else.
  Without the base vitality, `STA` would be 27, 27 and 28 and the HP sum 22 instead of 33.
  A second character confirms the base: its base is 15, where the first one's is 8.
  No 1-in-8 stat roll and no HP bonus fell in these steps, so this test adds nothing for either: they keep their [B] source and the evidence between the two game states.
- Spell charges: the two mages hold `[7, 9, 9, 9, 9, 9, 9, 9]`, the level-69 row of one group and the level-60 row of the other; level 1 reaches 9 through the level-1 base `magical_times1` of 2 [M, D].
  The Knight holds `[4, 4, 4, 0, …]` and the Ninja `[4, 4, 4, 4, …]`.
  A replace rule would give the Knight the level-69 row, all zeros, so only add then cap fits [M, D].

**Level and Experience in NeoFFSE.**
The game interface keeps `addtionalLevel` and `currentExp` consistent the way the game does [I].
An Experience edit sets `addtionalLevel` to the level the new experience reaches: the highest L with `total_exp(L) <= currentExp`, at most 99.
A Level edit sets `currentExp` to `total_exp` of the new level, the first experience point of that level.
The game interface accepts Level 1 to 99.
Expert mode (raw edits) keeps Level 0 to 99, links nothing and moves no stat: the storage bound of the Level is `MAX_LV`, not the general bound ([Bounds in the master data](#bounds-in-the-master-data)).
The app clamps typed text above 99 to 99 before it sends the edit.
A save whose stored Level is above 99 refuses the next game Level edit, the next game Experience edit and the next raw Level edit with `OutOfRange` naming the stored value, because the inverse of the edit would restore a Level outside the storage bound.
The save stays as it is, and every other field stays editable, raw Experience included.
A raw edit can leave the two fields apart.
With an Experience above the range of the Level, the menu shows the stored Level and takes the next-level figure from the Experience [G, edited copy].
The next battle that gives the character experience sets the Level to the level the Experience gives, with the growth steps of each level gained when it is higher and no stat change when it is lower [M, edited copy].

A Level or Experience edit that raises the level also plays one growth step per level gained, from the stored level plus 1 to the new level [I].
The steps use the character's current job and draw their random parts from the editor's own generator, so two edits of the same save can give different stats, as two battles do.
The edit writes the raw sums, one `Set` per field whose final value differs, under `StorageOnly`: a stat can pass its cap of 99, as the game stores it [B].
The fields a step moves are `max_hp_gain`, the seven stats, `accuracy_gain`, `magic_evasion_gain` and the eight `charges_max_gain` values.
A lower or equal level moves no stat.
A job without a growth row, or a `characterStatusId` without a base row, refuses the whole edit and changes nothing.

### Equipment slots

FF I characters hold five equipment slots, keys 1 to 5 of `equipmentList` in that order.
An empty slot holds the placeholder `contentId` of its slot, not 0 [M, 1 entry; D].

| Key | Slot (Go editor label) | Go table | Placeholder |
|---|---|---|---|
| 1 | R. Hand | `Weapons` | 62 |
| 2 | L. Hand | `Shields` | 130 |
| 3 | Body | `Armors` | 131 |
| 4 | Head | `Helmets` | 132 |
| 5 | Arms | `Gloves` | 133 |

- **Keys and slots.**
  The keys read `[1,2,3,4,5]` in that order for every FF I character [M].
  The Go editor labels the rows by position in `values` (`coreEquipment.go`) [C].
  34 of 40 slot entries (slot 1 and Quick Save, 20 each) resolve to the Go table of their slot [M].
  The other 6 hold IDs that the Go tables lack and the game names: Cat Claws (67), Sasuke's Blade (100) and Masamune (103) in the right hand, Flame Shield (166) in the left hand, each inside the ID range of its slot's table [M, D].
  FF I has no accessory slot.
- **Empty slots.**
  One slot entry holds a placeholder: 130 in a left hand of the Quick Save [M, 1 entry].
  The game holds placeholder rows for 62, 131, 132 and 133 too [D]; no sample holds one in a slot.
- **`count` copies the inventory count.**
  In FF I, `count` equals the inventory count of the same `contentId` in 40 of 40 slot entries, placeholder 130 included (count 9, inventory 9) [M].
  In FF II, 54 of 75 entries match and all 21 others sit below the inventory count [M].
  Reading: the game copies the inventory count when a character equips and does not refresh it [I].
  Whether the game reads `count` at all is open.
- **Equip and unequip in NeoFFSE.**
  The game interface edits a slot the way the game does [I].
  Equip writes the slot's `contentId`, makes sure the inventory row of the item holds at least one per wearer (it inserts the row with the wearer count, or raises the count to it), then copies the inventory count into the slot `count`.
  Unequip writes the slot's placeholder, copies the count of the placeholder's inventory row into the slot `count` (when the list holds that row) and keeps the item in the inventory.
  The game interface refuses to set a slot `count` by hand, since the equip plan owns it [I].
  The raw interface writes any of these fields alone.

### Abilities

An ability carries two IDs.
`abilityId` points into the game's ability table (`ability.csv`); `contentId` points into the content table that items and equipment share, at the type-4 row whose `type_value` is the `abilityId` [D].
Each game links the two by a fixed offset for its spells, with exceptions, and a writer must fill both.

Ability object: `{"abilityId":int,"contentId":int,"skillLevel":int}`.

| Game | `contentId − abilityId` | Tag |
|---|---|---|
| FF I | 208 | [M] 47 of 47 abilities and slot spells, slot 1 and Quick Save; [D] 95 of 96 `ability.csv` rows, the exception being ability 100 at content 310 (offset 210) |
| FF II | 207 for spells; ability 711 maps to content 937 (offset 226) | [M] 35 spells at 207; each FF II character holds ability 711 |
| FF III | 201 | [C] |
| FF IV | 310 | [C] |
| FF V | 266 | [C] |
| FF VI | 330 | [C] |

The Go editor applies the offset only to an ability whose `contentId` is 0 [C].
Ability 711 is missing from its FF II table, which stops at 644 [C].

Character keys that reference abilities:

| Key | Holds | Rule |
|---|---|---|
| `abilityList` | ability objects as JSON strings | one entry per `abilityId` [M] |
| `additionOrderOwnedAbilityIds` | content IDs | the content IDs of `abilityList`, same order, in all 35 characters of the six slots [M] |
| `sortOrderOwnedAbilityIds` | content IDs | empty or a subset of the owned ones; only FF II had entries [M]; the Go editor never edits it |
| `abilityDictionary` | dictionary | FF II: key `3` maps to `{"target":[ability JSON strings]}`; FF I: empty [M]; meaning not measured |
| `skillLevelTargets` | dictionary | FF II: keys 0 to 9 with ints; FF I: empty [M]; meaning not measured |
| `learningAbilitys` | list | empty in FF I and FF II [M] |

`userData → learnedAbilityList` holds ability IDs.
In FF I it equals the union of the party's `abilityId`s as a set (47 IDs in slot 1 and the Quick Save), and its order is not ascending [M, 4 slots].
In FF II it does not equal that union, and its meaning is unknown [M].

**FF I spell slots** [M].
`abilitySlotDataList → target[]` holds 8 JSON strings, one per spell level: `{"level":1..8,"slotInfo":"<JSON>"}`.
`slotInfo` is `{"keys":[1,2,3],"values":[v1,v2,v3]}`, each value either `""` (empty slot) or an ability JSON string.
In both FF I files, each spell of `abilityList` sits in a slot and each slot spell sits in `abilityList`.

FF I writer rule [C, `375f916`; consistent with M]: for each spell in a slot, set `contentId = abilityId + 208`, list it once in `abilityList`, and list its content ID once in `additionOrderOwnedAbilityIds`.

**FF I spell level and school** [M, 47 of 47 slot spells; D, all 64 spells].
`abilityId` fixes both.
The two spellcasters hold 47 spells in all, `abilityId` 5 to 68, every one in `Abilities`.

- Level = (`abilityId` − 5) / 8 + 1, with integer division.
  It equals the `level` of the `abilitySlotDataList` entry that holds the spell.
- In each level, offsets 0 to 3 (`(abilityId − 5) % 8`) hold the White Wizard's spells (24 of 24) and offsets 4 to 7 the Black Wizard's (23 of 23).
  The names agree: 5 to 8 are Cure, Protect, Dia and Blink, and 9 to 12 are Fire, Focus, Sleep and Thunder [C].
- `ability.csv` holds the same rule for all 64 spells: `ability_lv` equals the level above, and `type_id` is 1 for offsets 0 to 3 and 2 for offsets 4 to 7, without exception [D] ([Ability, job and command tables](#ability-job-and-command-tables)).
- Knight and Ninja hold no spell, so the corpus does not test their access to white or black magic.
- `Abilities` lists 5 to 68 as spells and 69 to 94 as monster abilities [C], and the `type_id` of `ability.csv` agrees [D]; no character holds an ID above 68 [M].

**`skillLevel`.**
FF I: `0` [M].
FF II: spell proficiency, 0 to 456 measured; the Go editor edits it as a level [M, C].
FF VI: learning percentage, 100 means learned [C].

### Party

`userData → corpsList` lists the active party; keep `characterId` values pointing at existing character `id`s.

- `corpsList → target[]`: 4 JSON strings `{"id":1,"characterId":n}` in FF I, FF II, FF III, FF V and FF VI, 5 in FF IV; `id` is 1 in each entry in FF I and FF II [M].
  NeoFFSE lists the party in the order of this list [I].
  The measured files do not show whether that order is the battle order.
  The editor view lists every character in the order of `ownedCharacterList`, in the party or not [I].
- `corpsSlots`: dictionary with keys `0`, `1`, `2`; each value is a list with the same entry shape [M].
  In the six slots all three lists equal `corpsList` [M].
  FF VI uses several parties [C].
- `userData.corpsSlotIndex`: `0` in all six slots [M].
- `mapData.playableCharacterCorpsId`: 0 to 3 [M]; meaning not measured.
- `userData → releasedJobs → target`: FF I 1 to 12, FF II 4 entries [M].

### Inventory

`userData → normalOwnedItemList → target[]` holds `{contentId, count}` JSON strings in the game's own order; the sort list holds content IDs, never row numbers.

- Normal items: 47 to 56 entries, no duplicate `contentId`, no `count` of 0, not sorted by ID [M].
  NeoFFSE refuses a file that repeats a `contentId` in either item list [I].
  `MAX_ITEM` (99) is the master-data cap on this list's `count` [D, I]: the cap exists in the data [D], and its scope follows from the key items below holding one each [I] ([Bounds in the master data](#bounds-in-the-master-data)).
- **No key item in the normal list.**
  None of the 221 rows of `normalOwnedItemList` in the four measured FF I slots holds a content ID from 45 to 61 [M, 4 slots].
  NeoFFSE resolves the row keys of `inventory` in the item and equipment tables only, and the row keys of `key_items` in the key item table [I].
- `importantOwendItemList`: same shape.
  Every measured row holds `count` 1 [M].
  In FF I it holds 9 or 10 entries, IDs 45 to 61, all in `ImportantItems` [M].
  In the game's data these IDs are the key items, `item.csv` `type_id` 2 [D].
  The game holds one of each key item and never shows a key-item `count` above 1 [I], so `MAX_ITEM` does not apply to this list.
  A format that edits the way the game does keeps every key-item `count` at 1 [I]: NeoFFSE's game interface refuses to set a key-item `count` and to insert a key item with another `count` [I].
- **The inventory includes equipped items.**
  In FF I, each equipped `contentId` appears in `normalOwnedItemList` with a count at least the number of characters who equip it.
  A ring worn by all 4 characters has count 7 in slot 1 and 6 in the Quick Save [M].
  No equipped ID is missing or under-counted [M].
  NeoFFSE's game interface keeps that floor: it refuses an inventory `count` below the number of wearers, in a `count` change and in an insert [I].
- **The inventory holds the five placeholders.**
  In FF I, 62, 130, 131, 132 and 133 sit in the list in both game states, as its first five rows, with counts 5, 9, 5, 9 and 9 [M].
  What those counts mean is unknown, so NeoFFSE's game interface refuses every edit of a placeholder row: a `count` change, an insert and a removal [I].
- **Removal in NeoFFSE.**
  The game interface removes a row only when no character wears its item and the row is no placeholder [I].
  Both interfaces refuse a row whose text the editor cannot rewrite byte for byte (other spacing, key order or escapes) and a row whose stored `count` lies outside the storage bound (0, a negative number, or above `i32::MAX`), because the inverse insert could not restore it.
  Such a row counts as locked [I].
- **IDs the Go tables lack.**
  The FF I list holds 5 IDs (4 in the Quick Save) that no Go table names: 67, 100, 103, 151 and 166.
  `content.csv` names all five ([Go editor tables](#go-editor-tables)) [M, D].
  Keep them as read.
- `normalOwnedItemSortIdList → target`: content IDs.
  FF II slot 1 and FF II Auto Save hold 23, each one an owned item; the other four slots hold none [M].
  The Go editor drops rows with `contentId` 0 and keeps the sort list unless the user resets it [C].
- `warehouseItemList`, `ownedMagicList`: empty in FF I and FF II [M].
- `ownedKeyWaordList → target`: FF II keywords, ints 917 to 931 [M]; content IDs [I].

### Party position and map

`mapData` holds the player's map, point and position.
A writer copies each key back, including keys it does not edit.

| Key | Type | Measured |
|---|---|---|
| `mapId` | int | 1 to 228 |
| `pointIn` | int | −1 to 3 |
| `transportationId` | int | 1 |
| `carryingHoverShip` | bool | |
| `playerEntity` | JSON | `{"position":{"x":f,"y":f,"z":f},"direction":int}`, direction 1 to 4 |
| `companionEntity` | string | `""` |
| `gpsData` | JSON | `{"mapId","areaId","gpsId","width":256,"height":256}`; `transportationId` in FF III, IV and VI only [M] |
| `moveCount`, `subtractSteps` | int | |
| `telepoCacheData`, `timerData` | string | `""` |
| `playableCharacterCorpsId` | int | 0 to 3 |
| `encountEnable`, `minimapEnable` | bool | present in all six slots |

[FF VI](#ff-vi) lists the FF VI keys.

### Transportation

`userData → ownedTransportationList → target[]` holds one JSON string per vehicle: 4 in FF I, 7 in FF II, 14 in FF III, 11 in FF IV, 19 in FF V, 8 in FF VI [M].
The count identifies the game ([Telling the games apart](#telling-the-games-apart)).

| Game | Slot files | Entries | `id` values, in file order |
|---|---|---|---|
| FF I | 4 | 4 | 1 to 4 |
| FF II | 4 | 7 | 1 to 7 |
| FF III | 2 | 14 | 1, 2, 3, 5, 6, 8 to 16 |
| FF IV | 2 | 11 | 1 to 11 |
| FF V | 2 | 19 | 1 to 15, 17 to 20 |
| FF VI | 2 | 8 | 1 to 8 |

- The ID sets differ pairwise, and so does the highest ID (4, 7, 16, 11, 20, 8) [M].
- The list does not track ownership.
  `enable` is `false` in every entry of every file, and 0 to 2 entries per file sit on a map (`mapId` not −1).
  The FF V files come from an early-game, one-character save and hold 19 entries [M].
- FF I's master data, `transportation.csv` (build 20249397), has 4 rows with IDs 1 to 4, the IDs the FF I saves hold [D].
  The tables of FF II to VI are unread, so their counts stay a reading of the saves [M].
- The count does not grow with progress within the measured range: late FF I saves hold 4 entries and early FF V saves hold the full 19 [M].
- Keys: `position` (`x`, `y`, `z` floats), `direction` (0 to 8), `id`, `mapId`, `enable`, `timeStampTicks` [M].
- An unplaced vehicle has `mapId` −1 and a position such as `(1000.0, 1000.0, -1000.0)` [M].
- The Go editor reads and writes the list entry by entry and never adds or removes one (`save/userData.go`, `models/core/transportations.go`) [C].
  In its FF VI menu, IDs 4 and 5 are Blackjack and Falcon (`ui/forms/editors/coreTransportation.go`) [C], consistent with an 8-row FF VI table.
- `timeStampTicks` counts .NET DateTime ticks: 100 ns units since 0001-01-01.
  The invented value `639083329294237651` reads 2026-03-05 18:42:09; the measured values fall in the date range of the saves [M, I].
  `0` means never set [M].
  Store it as i64.
  The Unix epoch is tick `621355968000000000` ([Known pitfalls](#known-pitfalls), 20).
  The time zone (local or UTC) is unknown.

### Counters and flags

| Key in `userData` | Type | Notes |
|---|---|---|
| `owendGil` | int | the game caps it at 999 999 (`MAX_GIL`) [D] |
| `totalGil` | int | `0` in all six slots |
| `playTime` | float | seconds; an invented example is `13621.814333377138` |
| `steps`, `escapeCount` | int | `0` in all six slots |
| `battleCount`, `openChestCount`, `saveCompleteCount`, `monstersKilledCount` | int | |
| `currentArea`, `currentLocation` | string | free text; `currentLocation` may be `""` |
| `owendCrystalFlags` | JSON list | 4 booleans in FF I and FF II |
| `configData` | string | `""` in all 14 game slots |
| `cheatSettingsData` | object | 9 keys: `isEnableEncount`, `expRate`, `gilRate`, `learningProficiencyRate`, `isHpIncreaseByFightCount`, `abpRate`, `magicProficiencyRate`, `parameterUpRate`, `magicLearningProficiencyRate`; FF III adds `jobProficiencyRate` |
| `isOpenedGameBoosterWindow` | bool | |

All rows [M], except the cap marked [D].

NeoFFSE shows `currentLocation` as the place of the save when it is not empty, and `currentArea` otherwise [I].
The file does not say which of the two the game shows.

### Time stamp and progress flags

`timeStamp` is a formatted local date whose layout varies by game; keep the string as read and write an edit in the same layout.

| Files | Layout | Invented example |
|---|---|---|
| FF I, all three slots | `M/d/yyyy h:mm:ss tt` [M] | `3/17/2026 6:42:09 PM` |
| FF II, all three slots | `dd/MM/yyyy HH:mm:ss` [M] | `17/03/2026 18:51:30` |
| FF III to VI, slot 1 and Auto Save | the FF I layout [M] | |

- FF II is the only game with the `dd/MM/yyyy` layout.
  FF I and FF II ran on the same machine on the same days and used different layouts [M].
  The Go code says the game uses the system culture [C]; the measurement leaves the cause open ([Open questions](#open-questions)).
- The Go editor parses `M/d/yyyy HH:mm:ss`, `dd.MM.yyyy HH:mm:ss`, `yyyy/MM/dd H:mm:ss`, `yyyy-M-d H:mm:ss` and ISO 8601 [C].

`dataStorage` keys [M]: `scenario` (128 ints), `treasure` (128 ints), `global` (16 ints), `area` (8), `map` (8), `selected`, `itemSelected`, `transportation` (empty).
`scenario` and `treasure` hold u32 values up to `4294967295`; the values look like bit sets [I].
FF VI stores the Cursed Shield fight count in `global[9]` [C].

### Bestiary

The bestiary file serves every slot of a game.
`monsterDefeats` maps monster IDs to defeat counts in first-defeat order, and `totalSubjugationCount` equals the sum of the counts.

- `scenarioFlags → target`: 1024 ints, each 0 or 1 [M].
  Meaning not measured; the Go editor never edits them.
- `monsterDefeats`: `{"keys":[ids],"values":[counts]}`, keys unique, counts ≥ 1, keys unsorted [M].
- `totalSubjugationCount` = sum of `values` [M].
  It differs from a slot's `userData.monstersKilledCount`, since one bestiary spans all slots [I].
- A complete FF I bestiary holds IDs 1 to 128, each once [M].
  The FF II file holds 75 IDs, the highest 118 [M].
- The Go editor's tables list boss variants above 128 (FF I 129, FF II 129 to 142) and treat them as counted under the base monster [C].
  No measured file holds an ID above 128 [M].
  In FF I the key 129 would be the second Vampire row; the other four repeated names (Lich, Marilith, Kraken, Tiamat) hold both of their keys, 120 to 127 [M, D].

**FF I key space.**
The keys of `monsterDefeats` are `monster.csv` IDs [M, D] ([Map and monster tables](#map-and-monster-tables)).
The evidence comes from one FF I bestiary file of 689 bytes:

- It holds 128 keys and 128 values, all keys distinct, from 1 to 128 without a gap.
- Read as monster IDs, the first ten keys in first-defeat order are the monsters of the opening area, then its boss (Garland, `boss` 1).
  Read as picture-book entries, keys 24, 22, 74, 44 and 106 name monsters of later regions, and the list holds no boss.
- Over all 128 keys in file order, the monster-ID reading leaves 428 order inversions in the region order of the picture book, out of 8 128 pairs.
  The picture-book reading leaves 2 467.
- `picture_book_group.csv` cannot be the key space: its group IDs equal the picture-book IDs up to 46, so it fails the same test.
- Key 128 is Chaos, with count 1.
  Key 61 (Vampire) holds count 1, and no key names monster 129, the boss Vampire.
  The data cannot say whether the game files the boss Vampire's defeat under 61 or never counts it ([Open questions](#open-questions)) [I].

**Display order.**
`picture_book.csv` (129 rows) lists monsters by region, not by monster ID: entry 4 is monster 117, entry 5 monster 22, entry 6 monster 74, and 7 entries have `id` equal to `monster_id`.
Entry 47 is monster 129.
Entry 129 is Chaos, with `status_display` 0, the lone entry with that value.
`picture_book_group.csv` (129 rows) folds entries 46 and 47, the two Vampires, into group 46 and so holds 128 groups [D].
The picture book gives the order in which the game lists monsters, so a list in the game's order sorts the keys by the entry whose `monster_id` is the key [I].

"Defeat all" rules of the Go editor [C, `models/core/bestiary.go`]:

1. Keep each existing entry and its count.
2. Append each missing ID with count 1, in ascending ID order, after the existing entries.
3. Skip IDs ≤ 0 and, in FF I and FF II, IDs above 128.
4. Before writing, drop entries with ID ≤ 0 or count ≤ 0, merge duplicate IDs by adding their counts at the first position, and recompute `totalSubjugationCount`.

Write the bestiary only when it loaded and changed.
Keep `scenarioFlags`, unknown keys and number text as read [C].

### FF VI

The saves show the FF VI key names in `mapData` [M].
The measured FF VI saves come from early game: they hold no esper and no filled `playerEntity` of another party.
The rules below come from the Go code [C].

- Espers: `userData → ownedMagicStoneList → target` lists owned esper IDs; a character's `magicStoneId` holds the equipped one.
- Magic: each spell sits in `abilityList` with `skillLevel` as the learning percentage; 100 means learned; `contentId = abilityId + 330`.
- Other parties: `mapData.otherPartyDataList` is a plain array of objects `{mapId, pointIn, playerEntity, playableCharacterCorpsId}`, with `playerEntity` a JSON string as in [Party position and map](#party-position-and-map).
  Related keys: `partyPlayableCharacterCorpsId`, `currentSelectedPartyId`.
- Cursed Shield fights: `dataStorage.global[9]`.

### Slot `configData`

`configData` keys, in order [M]: `buttonType`, `battleModeIndex`, `battleSpeedIndex`, `battleMessageSpeedIndex`, `isCursorMemory`, `isKeepAutoBattle`, `messageSpeed`, `isLeftActionIcon`, `isLeftVirtualPad`, `isLeftMenuCommand`, `isLeftBattleCommand`, `reequipIndex`, `analogModeIndex`, `isAutoDash` (an int, `1`).
The keys are the same in all six games.
No slot carries volume or brightness keys, and the Go struct's optional ones never occur; those settings live in the system settings file [M].

## Write rules

A writer changes the keys the user edited and nothing else, re-encodes the container as in [Pseudo-code](#pseudo-code), and goes through the safe write pipeline of [Save safety](../architecture.md#save-safety).

1. Write the BOM back when the file had one [C].
   Write back the trailer of the decoded container.
   The game ends its files with CRLF and loads a file without it [G].
2. Terminate the DEFLATE stream with a final block, pad with zeros to a multiple of 32, and use the key and IV of [Cipher](#cipher).
3. Re-serialize only the string documents that hold an edit, and their parent strings up to the slot.
   Leave every other string byte for byte as read.
4. Serialize in the game's style: compact, raw UTF-8, floats with a decimal point, original key order and spellings.
   Go's encoder escapes `<`, `>` and `&` as `\u003c`, `\u003e` and `\u0026` [M]; the game wrote no `\u` escape in the measured files.
5. Set `id` to the slot number of the target file ([Slot files](#slot-files)) when you write a slot's data under another slot's name [M, two editor files].
6. Leave the slot index as read when you write into a slot with no file: the load menu lists a slot file whose flag is `false` ([Other files in the folder](#other-files-in-the-folder)) [G, FF I and FF II].
7. Fill both ability IDs ([Abilities](#abilities)) and keep the FF I lists consistent.
8. Keep `timeStamp` as read unless the user edited it; write an edit in the same layout [C].
9. Write the bestiary only when it loaded and changed; leave an unreadable bestiary untouched [C].
10. Writing a slot and the bestiary takes two writes; a failure between the two leaves a new slot with the old bestiary [C].
11. Back up every file you replace before the first write.
    The Go editor copies them to `<config dir>/backups/FF<n>/<YYYYMMDD-HHMMSS>/`, next to its own config file, with a `backup.json` manifest, and keeps 20 backups per game [C].
    [Save safety](../architecture.md#save-safety) gives NeoFFSE's layout.
12. Steam Cloud may report a conflict on the next launch; the player must keep the local files ([Save safety](../architecture.md#save-safety)).

## Known pitfalls

1. `rijndael.Decrypt` deletes stream zero bytes with the padding; inflate fails (FF II Quick Save, slot index of FF I and FF II; no FF III to VI file) [M, `6f8f74a`].
2. Files the Go editor's encoder writes can end in a zero byte that pitfall 1 deletes [M, two editor files, `6f8f74a`].
3. The Go editor's loader ignored base64 errors and carried on with empty data [C, `6f8f74a`].
4. A bestiary "eof" error-string hack dereferenced a nil error when the bestiary decoded fine [C, `6f8f74a`].
5. An AES library cannot decrypt the files: the block is 256 bits ([Cipher](#cipher)).
6. The library's `Encrypt` pads in place and can write into the caller's buffer [C, `codec.go`].
7. Struct-based re-serialization drops unknown keys: `minimapEnable` reset on each save, and five FF III and FF V keys with no struct field vanish ([Keys that differ per game](#keys-that-differ-per-game)) [M for presence, C, `6ef3bc7`].
8. Re-serialization turns `58.0` into `58` and `&` into `\u0026` [M, Go encoder].
9. An f64-only JSON parser rounds `timeStampTicks` (above 2^53) [M].
10. Values exceed i32 (u32 flags in `dataStorage`, i64 ticks), while the Go editor's number fields clamp to i32 [M, C `6ef3bc7`].
11. Map "Misc" fields (`mapId`, `pointIn`, `transportationId`, `carryingHoverShip`) reverted on save [C, `6ef3bc7`].
12. FF VI other parties: edits went to a copy, and an inverted error check wrote them only when encoding failed [C, `6ef3bc7`].
13. A stale by-ID ability map created duplicate abilities on add [C, `d8a6c6f`].
14. FF VI toggles and reselecting a character appended duplicate or unlearned spells [C, `d8a6c6f`].
15. FF I abilities added in the editor kept `contentId` 0 [C, `375f916`].
16. FF I slot spells went into `additionOrderOwnedAbilityIds` twice [C, `375f916`].
17. FF II ability 711 breaks the +207 rule; it maps to 937 [M].
18. "Reset Sort" wrote row numbers `1..N` into `normalOwnedItemSortIdList` [C, `6ef3bc7`].
19. Time stamps: a `0` prefix, one en-US layout, and a rewrite of stamps the editor could not parse [C, `6ef3bc7`].
20. `NowToTicks` is 112 days off; the .NET epoch offset is `621355968000000000` [C].
21. The Go editor rewrote and backed up the bestiary on each save, even unchanged [C, `6f8f74a`, `68e6a80`].
22. Character lookup by name opened the wrong character; use `id` [C, `d8a6c6f`].
23. Switching between PC and PlayStation kept file names and the codec of the other mode [C, `7a83b22`].
24. A switch to an empty folder kept slot buttons bound to the old folder, and "save" overwrote the real game save [C, `7a83b22`].
25. A crash during a non-atomic write left an empty or partial slot [C, `375f916`].
26. An empty nested document is `""`; writing `"{}"` changes the file [M].
27. The Go editor labels `addtionalMaxHp` "HP Max", but the field is a gain over a base ([Characters](#characters)) [M, C].
28. The Go editor's content tables lack 8 content IDs of the game (4, 67, 100, 103, 104, 105, 151, 166) and disagree with it on 5 entries (76, 101, 102, 122 and the second 165).
    `Shields` lists 165 twice, as Ice Shield and Flame Shield; the game numbers Flame Shield 166, and the editor's ID-to-name map keeps the later 165 entry ([Go editor tables](#go-editor-tables)) [D, C].
29. The Go editor's `Abilities` swaps 13 and 15 (Invis and Blindna) and 66 and 67 (Warp and Stop), and its `Commands` names 3 "Flee" and 4 "Row" where the game has Magic and Items ([Go editor tables](#go-editor-tables)) [D, C].
30. Names repeat in the game's own files: five monster name keys serve two rows each, map labels repeat, and two `area.csv` rows read "Sunken Shrine".
    Key every table by ID ([Map and monster tables](#map-and-monster-tables)) [D].
31. A loose match on `message_assets_all_` finds `common_message_assets_all_<hash>.bundle` too, the UI sprite bundle ([Game master data](#game-master-data)) [D].

## Open questions

Each point needs a real save or an in-game test.

1. FF III to VI: slot names 2 to 20 and the Quick Save name (FF I and FF II: slot names 3 to 20), the ability offsets, FF VI espers and other parties, and how the game writes non-ASCII text in FF III, V and VI.
   The measured saves come from early game.
2. Does the game load a file without the BOM?
3. Does the game load a float written as an integer (`58` for `58.0`)?
   The Go editor writes that form [M]; no in-game test covers it.
4. Does the game pad an aligned stream with a full block of zeros, and does it accept zero bytes of padding?
   None of the 34 files had a stream length divisible by 32.
5. The `timeStamp` layout: FF I and FF II used different layouts on one machine, and the setting that picks the layout is unknown.
6. `timeStampTicks`: local time or UTC?
7. Meaning of `clearFlag` against `isCompleteFlag` (the FF I Auto Save holds `clearFlag` 0 and `isCompleteFlag` 1), of the top-level `playTime` (`0.0` in all 14 game slots), of `pointIn`, `playableCharacterCorpsId`, `skillLevelTargets`, `abilityDictionary`, `scenarioFlags`, and the FF II `learnedAbilityList`.
8. Does the game accept an inventory item missing from a non-empty `normalOwnedItemSortIdList`?
9. The PlayStation variant: file layout after extraction, presence of a bestiary, BOM.
10. System settings `language`: value `3` in all six games; mapping not measured.
11. Does the game read equipment `count`, and what do the inventory counts 5 and 9 of the placeholders mean?
12. An FF I save from before the class change: which `jobId` does a character hold (1 to 6, if `characterStatusId` names the base class), and do the placeholders 62, 131, 132 and 133 fill bare slots?
    No measured file comes from a new game.
13. Does the game count the boss Vampire (monster 129) under key 61, or not at all?
    The FF I bestiary holds no key 129, and the master data does not say.
14. Caps the master data omits: MP, the magic stat and the gain fields `addtionalMaxHp` and `addtionalMaxMpCountList`.
    An in-game test with an edited value would show whether the game clamps it.
15. The label the game shows for a map (area name and title joined), and the use of the `map.csv` rows 800 to 802.
16. Does a slot file from a new game, saved at the first save point, hold the full transportation list?
    The FF V files (one character, 19 entries) make a short or empty list unlikely [I], but no measured file comes from a new game ([Telling the games apart](#telling-the-games-apart)).
17. Do the `transportation.csv` tables of FF II to VI hold 7, 14, 11, 19 and 8 rows with the IDs the saves list?
    Reading them from an install would settle it and explain the gaps (FF III lacks IDs 4 and 7, FF V lacks 16).
18. A patch or a new build that adds or removes a vehicle row shifts that game's count, and the rule would refuse every save of the build.
    The error names the count it found so a report carries it.
19. Other builds and the PlayStation variant: every count comes from one Steam build, and another tool that rebuilds the list (adding or dropping entries) would defeat the rule.
    The Go editor does not [C].
20. Which game holds the measured `mapData.mapId` values up to 228, and does `mapId` reference `map.csv`?
    FF I's `map.csv` lists IDs 1 to 127 and 800 to 802 [D], and the link rests on the Go editor's code [C] ([Party position and map](#party-position-and-map)).
21. Maximum HP of a wounded character.
    All 8 measured character states sit at full HP, so `currentHP` equals `min(base + addtionalMaxHp, MAX_HP)` there.
    The menu readings cannot exclude a menu whose maximum equals `currentHP`.
    The match of the base on 7 states makes that reading unlikely [I].
    A saved character with lowered HP, read in the menu as a current and maximum pair, would settle it.
22. The next-level display at level 99.
    `exp_table.csv` has no row for level 100, and no measured save holds a character at level 99.
23. Which `game_constant_int.csv` keys cap the level, the experience and the experience setter?
    The game binary reads them through a string key that stayed unresolved; `MAX_LV` and `MAX_EXP` are the likely keys [I].
    The `GROWTH_CURVE_HP_RATE` row (8) is not read on the FF I path: the 1-in-8 roll is a literal in the code [B].
24. Which characters does a battle level?
    The game runs the level-up for the characters in its experience list (`BattleUtility.IsGetExp`).
    The filter was not read; living, non-petrified members are the likely rule [I].
25. Can a battle bonus still be active when a level-up runs?
    The vitality the HP rule uses also adds a temporary vitality and two bonus entries.
    FF I equipment adds no vitality [D], and the code does not show whether a battle effect lasts until the level-up.
26. Does `MAX_ABILITY_EVASION_RATE` cap `addtionalAbilityEvasionRate`?
    The field holds the growth of the magic defence ([Characters](#characters)), and the bound sits among the `MAX_` rows with no known save field ([Bounds in the master data](#bounds-in-the-master-data)).
    The code that reads the bound would settle it, or a save whose sum of base and gain passes the bound.
