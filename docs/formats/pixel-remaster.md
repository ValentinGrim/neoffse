# Pixel Remaster save format

A Final Fantasy I to VI Pixel Remaster (PR) save file is compact JSON,
compressed with raw DEFLATE, encrypted with Rijndael-256 in CBC mode and
written as base64 text. The container rules below hold in all six games.

## Scope and sources

The rules rest on 32 Steam save files the game wrote: 6 each for FF I
and FF II, 5 each for FF III to VI. Two more files, a slot 2 of FF I and
of FF II that KiameV's editor wrote, show how that editor encodes
([Slot 2 of FF I and FF II](#slot-2-of-ff-i-and-ff-ii)). The container
and top-level JSON rules hold in all six games. The content rules that
the early-game FF III to VI saves do not exercise (ability offsets, FF VI
espers and parties) come from the code of KiameV's Go editor and stay
[C].

The FF I domain rules ([ID spaces and tables](#id-spaces-and-tables),
[Equipment slots](#equipment-slots)) rest on two independent game states:
slot 1 and the Quick Save, four promoted characters each, from late game.
The Auto Save equals slot 1 in every field those rules measure, and slot 2
is the editor's copy of slot 1 [M]. Every [M] on an FF I character value
in those sections rests on 8 character states. The ranges quoted there are
observations, not bounds: no measured value comes from a new game (level 1,
bare hands, low gil).

### Games

| Game | Steam app ID | Game folder under `My Games` | Executable |
|---|---|---|---|
| FF I PR | 1173770 | `FINAL FANTASY PR` | `FINAL FANTASY.exe` |
| FF II PR | 1173780 | `FINAL FANTASY II PR` | `FINAL FANTASY II.exe` |
| FF III PR | 1173790 | `FINAL FANTASY III PR` | `FINAL FANTASY III.exe` |
| FF IV PR | 1173800 | `FINAL FANTASY IV PR` | `FINAL FANTASY IV.exe` |
| FF V PR | 1173810 | `FINAL FANTASY V PR` | `FINAL FANTASY V.exe` |
| FF VI PR | 1173820 | `FINAL FANTASY VI PR` | `FINAL FANTASY VI.exe` |

The Go editor holds the first three columns in `ui/forms/io/savedir.go` and
`saveSelect.go` [C]. Save folders copied from Proton installs confirm all
six folder names: `compatdata/<app ID>/…/My Games/<game folder>/Steam/<SteamID64>`
[M]. FF I has no "I" in its folder name. The `installdir` key of
each game's `appmanifest_<app ID>.acf` equals the game folder, so the
install folder under `steamapps/common/` carries the same name; each
install folder holds the executable above and one `<name>_Data` folder
[M, directory listings of six installs].

Platforms covered:

- **Steam PC**: Windows native and Linux through Proton, covered from
  [Files on disk](#files-on-disk) to [Known pitfalls](#known-pitfalls).
- **PlayStation `slotN.sav`**: the variant the Go editor opens
  ([PlayStation variant](#playstation-variant)). Its rules come from the
  Go code; no measured file backs them.

### Sources

| Source | Location | Used for |
|---|---|---|
| KiameV's Go editor | `final-fantasy-pr-save-editor` 1.3.1, fork branch `bestiary-and-fixes`, commits `6f8f74a` to `d4de261` on top of `a91b889` | codec, data model, pitfalls (commit messages explain each bug) |
| ID tables | `github.com/ValentinGrim/final-fantasy-pr-save-editor`, tag `v1.5.1` (`32d3e59`), MIT, © 2022 Travis Offtermatt: package `models/core/costs/ff1` and `models/nameValue.go`. Both are byte-identical to KiameV's last upstream commit `cea371fe99b4476e9ed419961eea04f7de605054` (repository archived), the original work | names and IDs of the FF I items, equipment, spells and jobs ([ID spaces and tables](#id-spaces-and-tables)) |
| Cipher library | `github.com/kiamev/ffpr-save-cypher` v1.0.0, MIT, © 2024 KiameV, a port of py3rijndael | key, IV, padding |
| Real saves | one Steam save folder per game, read-only copies | every [M] fact about the files |
| Steam's own files | `libraryfolders.vdf`, `appmanifest_<app ID>.acf`, `remotecache.vdf`, `loginusers.vdf` and the install folders of one Linux Steam install with two libraries; structure only, personal values masked | every [M] fact about roots, libraries and Steam Cloud |
| steamlocate | [steamlocate](https://crates.io/crates/steamlocate) 2.1.1, MIT: `src/locate/linux.rs`, `src/locate/windows.rs`, `src/library.rs` | Steam roots, registry keys, library parsing |
| Steam on Flatpak | [Flathub `com.valvesoftware.Steam` wiki](https://github.com/flathub/com.valvesoftware.Steam/wiki) | Flatpak data path and library overrides |
| Proton prefixes | [GamingOnLinux, GE-Proton 9-3](https://www.gamingonlinux.com/2024/04/ge-proton-9-3-released-with-a-new-option-to-help-with-modded-games) | prefix in the game's library; Steam Deck session |
| Windows folders | Microsoft documentation: [`KNOWNFOLDERID`](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid), [`SHGetKnownFolderPath`](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetknownfolderpath), [OneDrive known folders](https://learn.microsoft.com/en-us/sharepoint/redirect-known-folders) | `FOLDERID_Documents`, OneDrive redirection |
| Steam Cloud roots | [SteamCloudFileManager `ROOT_PATH_MAPPING.md`](https://raw.githubusercontent.com/Fldicoahkiin/SteamCloudFileManager/master/ROOT_PATH_MAPPING.md), GPL-3.0; notes read, no code taken | root 2 is `WinMyDocuments` |
| In-game checks | edited files loaded in the game | every [G] fact |

The 34 measured files, sizes in bytes. "Editor" marks a file that
KiameV's editor wrote ([Slot 2 of FF I and FF II](#slot-2-of-ff-i-and-ff-ii)):

| File (first characters) | Role | FF I | FF II | FF III | FF IV | FF V | FF VI |
|---|---|---|---|---|---|---|---|
| `ookrbATY…` | slot 1 | 8 157 | 96 049 | 87 261 | 80 773 | 62 469 | 77 317 |
| `vgU2wnua…` | slot 2 | 7 855 (editor) | 95 835 (editor) | absent | absent | absent | absent |
| `7nCxyzTw…` | Auto Save | 112 305 | 63 749 | 88 197 | 80 005 | 61 533 | 35 589 |
| `Rl18osV3…` | Quick Save | 59 185 | 57 181 | absent | absent | absent | absent |
| `dp3fS2vq…` | bestiary | 689 | 561 | 177 | 177 | 177 | 221 |
| `2E4HxIyL…` | slot index | 133 | 133 | 133 | 133 | 133 | 133 |
| `V=SfwyU3…` | system settings | 861 | 861 | 861 | 861 | 861 | 861 |

The 32 game files end with CRLF; the two editor files do not [M].

### Confidence tags

| Tag | Meaning |
|---|---|
| [M] | Measured on the 32 game-written Steam files (FF I to VI) or, for roots, libraries and Steam Cloud, on one Linux Steam install; "[M, two editor files]" marks a measurement on the two editor-written files |
| [G] | Verified in game |
| [C] | Taken from the code, tests or commit messages of KiameV's Go editor, or from the code or documentation of another source named beside the tag; not measured |
| [I] | Inferred from measurements; consistent with the data, not proven |

## Files on disk

A game keeps one save folder per Steam account. The folder holds up to
22 slot files with fixed names, one bestiary shared by all slots, two
more game files and one Steam file.

### Save folder

| Install | Folder |
|---|---|
| Windows | `<Documents>\My Games\<game folder>\Steam\<SteamID64>\` [C] |
| Linux, Proton, native Steam | `~/.local/share/Steam/steamapps/compatdata/<app ID>/pfx/drive_c/users/steamuser/Documents/My Games/<game folder>/Steam/<SteamID64>/` [M for all six games] |
| Linux, Proton, Flatpak Steam | the same path under `~/.var/app/com.valvesoftware.Steam/.local/share/Steam/` [C] |
| Any Steam library | `<library>/steamapps/compatdata/<app ID>/…`: the prefix can sit in a library other than the one that installs the game ([Steam roots and libraries](#steam-roots-and-libraries)) [M, C] |

- `<SteamID64>` is the 17-digit account ID, starting with `7656` [M].
  Each of the six `Steam/` folders holds one entry of that shape and no
  other [M]. To list the accounts without a Steam file, keep the
  subfolders of `Steam/` whose names match `^7656\d{13}$` [M for the
  pattern on six folders]. The rule works on both systems.
- With several account folders, the Go editor picks one that holds a
  slot file, then the most recent modification time [C]. A scanner can
  list all of them and leave the choice to the caller [I].
- Under Proton the Windows user is `steamuser` (GE-Proton 11-9 and Proton
  11.0-100) [M]. `My Documents` there is a symlink to `Documents` [M].
- On Windows `<Documents>` is the Known Folder `FOLDERID_Documents`,
  which `SHGetKnownFolderPath` returns for the current user [C,
  Microsoft]. OneDrive Known Folder Move redirects that folder to
  OneDrive [C], so the call returns the OneDrive path [I]. The PR games
  run on Unity (`<name>_Data` folders, `IL2CPP` thread names) [M], which
  resolves "My Documents" through the shell, so the saves follow the
  redirection [I]; a Windows install with Known Folder Move would settle
  it. The Go editor tries the Known Folder, then `%USERPROFILE%\Documents`,
  `%USERPROFILE%\OneDrive\Documents` and `%OneDrive%\Documents`, and
  drops duplicates after it resolves links [C]. On a non-English
  Windows the last fallback may miss, because the OneDrive folder name
  can differ from `Documents`; the Known Folder call does not depend on
  that name [I].
- Steam Cloud confirms the Windows shape. Each `remotecache.vdf` lists
  every file of the save folder as `My Games/<game folder>/Steam/<SteamID64>/<file>`
  with `"root" "2"`, which a community table maps to `WinMyDocuments`,
  the `Documents` folder [M, C].

### Steam roots and libraries

On Linux, find the saves in three steps: collect the Steam roots, read
every library each root lists, then probe `compatdata/<app ID>` in every
library. On Windows the save folder ignores Steam libraries and hangs off
`<Documents>`.

**Roots.** A candidate counts as a root only when it holds a `steamapps`
folder [C, steamlocate]:

| Root | Install |
|---|---|
| `~/.local/share/Steam`, or `$XDG_DATA_HOME/Steam` when set | native; the real install [M] |
| `~/.steam/steam`, `~/.steam/root` | symlinks to `~/.local/share/Steam` on a native install [M] |
| `~/.steam/debian-installation` | Debian `steam-installer` [C, steamlocate] |
| `~/.var/app/com.valvesoftware.Steam/.local/share/Steam`, with `.steam/steam` and `.steam/root` beside it | Flatpak [C, Flathub wiki, steamlocate] |
| `~/snap/steam/common/.local/share/Steam`, with `.steam/steam` and `.steam/root` beside it | Snap [C, steamlocate] |

Three of the native candidates point at one folder [M]. Canonicalize each
root and drop duplicates, or every save folder appears three times. The
Go editor and steamlocate both drop duplicates after they resolve links
[C]. On Windows the root comes from the registry value `InstallPath` under
`HKLM\SOFTWARE\Wow6432Node\Valve\Steam`, then under `HKLM\SOFTWARE\Valve\Steam`
[C, steamlocate]; save discovery does not need it.

**Libraries.** Read `<root>/steamapps/libraryfolders.vdf`. Steam keeps a
byte-identical copy at `<root>/config/libraryfolders.vdf`; the Go editor
and steamlocate read the `steamapps/` one [M, C]. The file is text VDF,
tab-indented, without a BOM [M]:

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

- The top key is `libraryfolders`. Each library is a child keyed `"0"`,
  `"1"`, and so on, and `"0"` carries the Steam root as its `path` [M, two
  libraries]. The real file holds more keys per library (`label`,
  `contentid`, `totalsize`) than this sketch shows [M].
- `apps` maps an app ID to a byte count. For the six PR games the value
  equals `SizeOnDisk` in the game's `appmanifest_<app ID>.acf` [M]. The
  lists matched the manifest files one to one on the measured install, but
  steamlocate lists apps from the manifests and notes that the VDF values
  may be stale [M, C].
- steamlocate keeps the children whose key parses as `u32` and reads
  `path` from each [C]. On Windows the VDF string doubles each backslash
  and the Go editor replaces `\\` with `\` [C]; a VDF parser that
  unescapes strings does the same job [I].
- `<library>/steamapps/appmanifest_<app ID>.acf` exists in the library
  that installs the game [M]. Its `AppState` block holds `appid`,
  `installdir`, `SizeOnDisk`, `LastOwner` (a SteamID64, personal data)
  and depot data [M]. `installdir` is the game folder of
  [Games](#games). Save discovery does not need the install folder; an
  asset provider or a process match does.

**Prefixes outside the game's library.** By default Steam keeps the
prefix in the library that installs the game [C, GamingOnLinux]: on the
measured install 41 of 43 prefixes belong to an app whose manifest sits
in the same library, and no app ID has a prefix in two libraries [M]. Two
cases break the rule:

- In the Steam Deck session (`-steamdeck`), Steam moves the default
  prefix path to the default install, so a game on an SD card or an
  external drive gets its prefix in the root library [C, GamingOnLinux].
  No Deck sample exists.
- Steam leaves a prefix behind when it uninstalls a game, so a save can
  outlive its install [M for one non-PR app, I for the PR games]. The
  two prefixes without a manifest on the measured install are
  `compatdata/0` and the prefix of an uninstalled game [M].

The rule: for each app ID, probe `compatdata/<app ID>` in every library
of every root, whatever library the manifest names, and report every hit
[M, C]. The Go editor already does this [C]. `STEAM_COMPAT_DATA_PATH`
overrides the prefix path per launch [C, community guides]; a prefix
moved that way lies outside every library and needs a folder the user
picks.

**Flatpak libraries.** The Flatpak app sees the host home under the same
path but redirects `XDG_DATA_HOME` to
`~/.var/app/com.valvesoftware.Steam/.local/share`; an extra library needs
a `--filesystem` override and keeps its host path [C, Flathub wiki]. The
`path` values of a Flatpak `libraryfolders.vdf` should therefore resolve
from the host [I]; a Flatpak install would settle it.

### Slot files

The game names each slot file with a fixed 44-character string that looks
like base64 and may contain `=` in the middle. The JSON `id` key of a
slot equals its slot number [M for 1 and 21 in six games and for 22 in FF I
and FF II; C for the rest].

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

The names come from `saveSelect.go` [C]. The Auto Save and slot 1 names
appear in all six games, the Quick Save name in FF I and FF II [M]. The
slot 2 name appears in the two editor-written files, and the game lists
those files [G]. The FF III to VI folders hold no Quick Save file because
the player made none in those games [I]. The names hold `+` and `=`,
which are legal on Windows and Linux; quote them in shells.

### Other files in the folder

| File | Size (FF I / FF II / FF III / FF IV / FF V / FF VI) | Content |
|---|---|---|
| `dp3fS2vqP7GDj8eF72YKqbT7FIAF=e7Shy2CsTITm2E=` | 689 / 561 / 177 / 177 / 177 / 221 | Bestiary, shared by every slot of the game ([Bestiary](#bestiary)) [M] |
| `2E4HxIyL+Mvaahk1GRNNOgKrO4vDSSm72kHLbVt71hg=` | 133 in all six | Slot index: `{"keys":[1,…,22],"values":[22 booleans]}` [M] |
| `V=SfwyU31ksiHpTxO7Rg2aBJrA18kSKvtF4jvDB4hRE=` | 861 in all six | System settings, 18 keys ([Documents of the other files](#documents-of-the-other-files)) [M] |
| `steam_autocloud.vdf` | 53 / 53 (FF I, FF II) | Steam Cloud marker, plain-text VDF with one `accountid` key [M] |

- The three game files use the container of
  [Container encoding](#container-encoding) [M].
- **Slot index.** `values` is `true` at the slots whose file the game
  wrote and `false` at the others: keys 1, 21 and 22 in FF I and FF II,
  keys 1 and 21 in FF III to VI [M]. The payload is 207 bytes in FF I
  and FF II and 208 in FF III to VI: one more `false` (5 characters) in
  place of a `true` (4) [M]. A slot file written by another tool leaves
  the index unchanged [M, two editor files]. The load menu lists such a
  file: slot 2 of FF I and FF II, flagged `false`, appears in the menu
  [G]. The index does not gate the menu's slot list, and what the
  game reads it for is unknown. The Go editor never reads or writes it
  [C].
- **System settings.** The file is byte-identical in all six folders
  [M]. The Go editor never touches it [C].
- **`steam_autocloud.vdf`.** Steam writes it, not the game. `accountid`
  equals `SteamID64 − 76561197960265728`, the account part of the folder
  name [M]. Leave it alone. The FF III to VI folders hold one beside the
  saves [M]; its size was not measured. Its presence marks a folder that
  Steam Auto-Cloud syncs, so a tool may use it to warn about Cloud
  conflicts [I].
- **Steam Cloud.** `userdata/<account ID>/<app ID>/remotecache.vdf` under
  the Steam root lists every file of the save folder except
  `steam_autocloud.vdf`, with 7, 7, 5, 5, 5 and 5 entries for FF I to
  FF VI [M]. Each key is relative to Cloud root 2, `WinMyDocuments`
  ([Save folder](#save-folder)). `<account ID>` is the 32-bit account ID:
  `SteamID64 − 76561197960265728` names an existing `userdata` folder
  [M]. The same folder also holds `userdata/0` [M].

### Slot 2 of FF I and FF II

The FF I and FF II folders each hold a slot 2 file that KiameV's editor
wrote, not the game. Five measurements separate each from the 32 game
files [M]:

| Evidence | Slot 2 (FF I, FF II) | The 32 game files |
|---|---|---|
| CRLF after the base64 | absent | present in 32 of 32 |
| Top-level `playTime` | `0` (integer form) | `0.0` in 14 of 14 slots |
| Dotted floats in the file | 1 (`userData.playTime`, non-integral) | 24 (FF I) and 33 (FF II) in the slots they copy |
| Slot index flag for slot 2 | `false` | matches the slot files present in 6 of 6 folders |
| Decoded content against slot 1 | equal except `id`, `userData.playTime` and, in FF II, 12 `skillLevel` values | |

Both streams end with a fixed-Huffman final block (`… 03 00` in FF I,
`… 07 00` in FF II), and `Decrypt` strips that last zero byte: each file
loses one stream byte ([Zero padding and the stripping
trap](#zero-padding-and-the-stripping-trap)). The BOM and the PNG
thumbnail match slot 1. The editor wrote slot 1 data under the slot 2
name with `id` set to 2 [I]. The two files show how the editor encodes
[M, two editor files]:

- It writes the BOM back and no CRLF.
- It writes integral floats as integers (`0`, not `0.0`).
- Its streams can end in a zero byte that `Decrypt` deletes.
- It sets `id` to the target slot number.
- It leaves the slot index untouched.

The game lists both files in the load menu although the slot index flags
slot 2 `false` [G].

### PlayStation naming

The Go editor names PlayStation slots `slotN.sav`, with N taken from
the `id` column of [Slot files](#slot-files): `slot1.sav` to
`slot20.sav`, `slot21.sav` for Auto Save, `slot22.sav` for Quick Save
[C]. It looks for the bestiary under the PC name `dp3fS2vq…` in the same
folder [C]. The measured set holds no PlayStation folder.

## Container encoding

A PC file is base64 text, with a UTF-8 BOM in front and CRLF at the end,
of a Rijndael-256-CBC ciphertext of a raw DEFLATE stream of compact UTF-8
JSON. The reference decryption deletes trailing zero bytes that belong
to the DEFLATE stream; a decoder must keep them.

### Layers

| Layer | Bytes | Tag |
|---|---|---|
| Prefix | UTF-8 BOM `EF BB BF` | [M] 34 of 34, editor files included |
| Text | one base64 line, standard alphabet, `=` padding | [M] |
| Line end | `0D 0A` | [M] all 32 game files |
| Ciphertext | Rijndael, 32-byte blocks, CBC | [M] |
| Plaintext | raw DEFLATE stream followed by 0 to 31 zero bytes | [M] |
| Payload | compact UTF-8 JSON, no BOM, no trailing newline | [M] |

The inflater stops at the final block and ignores every byte after it,
zero or not.

### BOM and line end

- The game writes the BOM and the CRLF on every file [M, 32 files].
- The Go editor's decoder records the BOM as a prefix [C]. Its encoder
  writes the BOM back and no CRLF [M, two editor files].
- The Go editor's decoder trims trailing space, tab, CR and LF before
  base64 [C].
- The game loads an FF I bestiary written by the Go editor's encoder,
  so it accepts a file without the trailing CRLF [G].
- NeoFFSE trims the same four bytes and keeps them as the trailer, byte
  for byte, up to 64 bytes. A longer trailer fails decoding, and encoding
  refuses a container that holds one. Any other byte, a form feed included,
  stays in the base64 text, and decoding fails on it.
- `decode` returns the BOM flag and the trailer, and `encode` writes back
  the BOM and the trailer of the container it receives. `Container::game()`
  holds the BOM and the CRLF the game writes. A container decoded from an
  editor file holds the BOM and no CRLF, and `encode` keeps it that way.
  The save pipeline of the `neoffse` facade (M4) chooses which container
  it passes; `neoffse-io` knows no format.

### Base64

- RFC 4648 standard alphabet (`A-Z a-z 0-9 + /`) with `=` padding [M]:
  text lengths are multiples of 4, and the files end in `=`, `==` or no
  padding as the ciphertext length requires.
- Text length is `4 × ceil(len(cipher) / 3)`. Example: 96 bytes of
  ciphertext give 128 characters, plus 3 bytes of BOM and 2 of CRLF:
  the 133-byte slot index [M].
- The Go editor's decoder appends missing `=` before decoding [C]. Before
  `6f8f74a` it ignored base64 errors and went on with empty data [C].
- NeoFFSE refuses a CR or LF inside the text. Go's `encoding/base64`
  decoder skips them, and no measured file holds one [M].
- NeoFFSE refuses a text longer than 44 739 244 bytes (`MAX_TEXT_LEN`)
  before it decodes. The largest measured file is 112 305 bytes.

### Cipher

| Parameter | Value |
|---|---|
| Algorithm | Rijndael, 256-bit block, 256-bit key, 14 rounds |
| Mode | CBC, fixed IV |
| Key (32 bytes, hex) | `61090700b9c3b8b9421599e79ca57b87bea932d3797bad7663ed4dde0a943cc5` |
| IV (32 bytes, hex) | `0224f9771fdb6e0e3bd508d7b795bf2e0cbd176942680a637b12bc6273db2ebb` |

- AES fixes the block at 128 bits, so an AES library cannot decrypt
  these files. NeoFFSE ports Rijndael-256 from the Go library.
- CBC: the encryptor XORs the first plaintext block with the IV and each
  later block with the previous ciphertext block [C].
- The Go library packs each 4-byte column big-endian
  (`b0<<24 | b1<<16 | b2<<8 | b3`) and ships both expanded key schedules
  (`ke` and `kd`, 15 round keys of 8 words) as constants. `ke[0]` is the
  key: `ke[0][0] = 1627981568 = 0x61090700` [C]. Both schedules follow
  from the key, so an implementation may expand them at run time
  instead of embedding them.
- With a 256-bit block (Nb = 8), the rows of the state shift by 0, 1, 3
  and 4 columns in encryption and by 0, 7, 5 and 4 in decryption. The Go
  library holds these as `shifts[2]` [C], and the Rust port reproduces
  its test vectors with them.
- One key and one IV serve every game, slot, bestiary and settings file
  [M: all 34 files decrypt with them].
- With a fixed key and IV, equal plaintexts give equal files: the
  settings file is identical in all six games, and the slot index file is
  identical in FF I and FF II and again in FF III to VI [M].

### Zero padding and the stripping trap

**Padding.** The encryptor appends zero bytes up to the next multiple of
32 and adds none when the length is already a multiple of 32
(`padder.Encode`) [C]. Game files carry 2 to 30 bytes of padding; none
of the 32 carries 0 [M].

**Stripping.** `rijndael.Decrypt` returns `padder.Decode(plain)`. That
function scans the last 31 bytes from the end and drops each zero byte
until it meets a non-zero byte, so it deletes up to 31 bytes [C]. It
cannot tell padding from data. A raw DEFLATE stream often ends with
`0x00` (for example the 7-bit end-of-block code of a fixed-Huffman
block), and the function deletes that byte too. Inflating the shortened
stream fails with `unexpected EOF`.

Measured cases [M]:

| File | Ciphertext | DEFLATE stream | Padding | Bytes `Decrypt` removed | Stream bytes lost |
|---|---|---|---|---|---|
| FF II Quick Save | 42 880 | 42 854 | 26 | 27 | 1 |
| Slot index, FF I and FF II | 96 | 83 | 13 | 14 | 1 |
| FF I slot 2 (editor) | 5 888 | 5 875 | 13 | 14 | 1 |
| FF II slot 2 (editor) | 71 872 | 71 869 | 3 | 4 | 1 |
| The other 29 game files | | | | equal to the padding | 0 |

No FF III to VI file loses a stream byte. A decoder must still keep
every decrypted byte: the loss depends on the last stream byte, and the
FF II Quick Save, the FF I and FF II slot index and both editor files
lose one each.

Before commit `6f8f74a`, the Go editor could not open the FF II Quick
Save, and KiameV's release disables the bestiary after the same failure
[C, `6f8f74a`]. Streams the Go editor's encoder writes with newer Go
versions hit the same bug [C, same commit]; both editor-written slot 2
files lose one stream byte [M, two editor files].

**Fix.** After `Decrypt`, append `len(cipher) − len(plain)` zero bytes,
then inflate. The inflater stops at the final block and ignores the
zeros after it, as the game does [C, `codec.go`]. A port whose decryption
never strips returns all `len(cipher)` bytes and needs no restore step.

**Fallback.** A stream without a final block (sync-flushed only) runs
into the padding, which an inflater reads as a stored block with a bad
LEN/NLEN pair. The Go editor's codec then inflates the stripped bytes and
accepts the result only when inflation hit the end of the input and the
output is valid JSON [C, `TestDecodeStreamWithoutFinalBlock`]. NeoFFSE
accepts the stripped view when inflation consumed all of it without
reaching a final block and the inflated bytes pass a shape check: they
start with `{`, brackets balance outside strings, the depth first returns
to zero at the last byte and no string stays open. The check does not
match a `[` with a `]`; full JSON validation belongs to the document
layer. None of the 34 files needs this path [M].

### Raw DEFLATE

- The stream is raw RFC 1951 DEFLATE: no zlib header, no gzip header,
  no checksum [M].
- Each stream ends with a block whose BFINAL bit is set, and holds no
  `00 00 FF FF` sync-flush marker or stored block [M, 34 files]. Game
  streams use dynamic-Huffman blocks; the FF II Quick Save and the two
  editor files end with a fixed-Huffman final block [M].
- The game's compressor differs from zlib on large inputs. No tested
  zlib setting reproduces a slot or the settings file [M]: levels 0 to 9
  with memLevel 8 or 9 on all files, and a grid of levels 1 to 9,
  memLevel 1 to 9, windowBits 9 to 15 and two strategies on FF III and
  FF VI. FF II slot 1 gives 72 003 bytes from the game and 71 913 from
  zlib level 6 [M].
- Small streams can match: the slot index (207 bytes in FF I and FF II,
  208 in FF III to VI) at level 9, the FF III to V bestiaries at levels
  7 to 9 and the FF VI bestiary at level 7 [M]. The FF I and FF II
  bestiaries match no setting.
- zlib does not reproduce the game's bytes, so a round-trip test
  compares the inflated JSON text.
- The game accepts a stream from Go's `compress/flate` at level 6,
  closed with a final block [G, FF I bestiary].
- JSON payloads of 80 to 235 KB compress to 6 to 84 KB in slot files
  [M].
- NeoFFSE refuses an inflated payload above 16 MiB (`MAX_PAYLOAD_LEN`).

### JSON text

The payload is compact JSON: no space or newline between tokens, no
trailing newline [M, 34 files]. Strings hold raw UTF-8 (`é` as `C3 A9`),
with no `\u` escape and no escaped `/` [M]. Only `\"` and `\\` escapes
occur, and no raw control character [M]. FF III, V and VI saves hold
ASCII text only, so they cannot show how the game writes a non-ASCII
character.
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
    stream = raw_deflate(json)                       # ends with a BFINAL block
    padded = stream + zero bytes up to a multiple of 32   # none when aligned
    cipher = rijndael256_cbc_encrypt(padded, KEY, IV)
    text = base64_std_encode(cipher)                 # with '=' padding
    return (EF BB BF if container.bom) + text + container.trailer
        # the trailer is whatever the container holds: the game's container
        # holds CR LF, a container decoded from an editor file holds nothing
```

### PlayStation variant

The Go editor reads a `slotN.sav` file as the JSON text of
[Top-level JSON document](#top-level-json-document), with no BOM,
base64, cipher or DEFLATE, and writes it back the same way [C]. It reads
the bestiary file the same way [C]. PS4 and PS5 encrypt saves with
console keys ([Goals and scope](../architecture.md#goals-and-scope)), so
these files must be what an external tool extracts from the console. The
measured set holds none.

## Top-level JSON document

A slot holds one object with ten keys. Four of them (`userData`,
`configData`, `dataStorage`, `mapData`) hold whole JSON documents
serialized into strings, and those documents nest more of the same:
seven levels deep in FF I, six in FF II to VI. An editor parses a string document, edits it and
re-serializes it in the game's style, and leaves each untouched string
as it read it.

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

The same keys hold documents in all six games, and every string
document is an object, never a bare array [M].

- Each string level doubles the escaping. A quote at depth n appears as
  `2^n − 1` backslashes and a quote [M, 34 files]. FF I spell slots
  reach depth 7, 127 backslashes; FF II to VI reach depth 6, 63
  backslashes [M].
- `userData.cheatSettingsData` is a plain nested object, not a string
  [M].
- An absent document is an empty string `""`, never `"{}"` or `"[]"`:
  see `companionEntity`, `telepoCacheData`, `timerData`,
  `userData.configData`, empty spell slots, `mapData.rtsData` and, in FF
  VI, `playerEntity` in each `otherPartyDataList` entry [M]. Keep `""`
  as `""`. `rtsData` and the FF VI `playerEntity` occur only as `""` in
  the measured files; their filled form is [C].

### Container shapes

The game builds documents from two shapes [M]:

| Shape | JSON | Notes |
|---|---|---|
| List | `{"target":[…]}` | elements are ints, booleans or JSON strings |
| Dictionary | `{"keys":[…],"values":[…]}` | parallel arrays of equal length; values are ints or JSON strings |

An object stored in a list is a JSON string, so a list of items reads
`{"target":["{\"contentId\":62,\"count\":5}", …]}` [M].

### Numbers

- Floats keep a decimal point: `58.0`, `1.0`, `0.0` [M]. In slot 1 of
  FF III to VI, every float but `userData.playTime` is integral and
  keeps `.0`. Go's JSON encoder writes `58` and `1`, so the Go editor
  rewrites each float it re-serializes as an integer [M, measured on
  `go-ordered-json` and two editor files].
- No number uses an exponent or a leading zero [M].
- The game writes doubles with up to 17 significant digits, not always
  in the shortest round-trip form [M]. Of the non-integral
  `userData.playTime` values, 4 carry 17 digits whose last digit differs
  by 1 or 2 from the shortest form of the same double; both texts parse to
  the same double [M]. Invented values show the property: `13621.814333377138`
  and `13621.814333377139` parse to one double, and the second is its
  shortest form. A serializer that prints a parsed double rewrites those
  4 values, so keep the original text of each number you do not edit. The
  correctly rounded 17-digit form equals the shortest form, so the game's
  last digits are not correctly rounded [M].
- Integers exceed the i32 range. `dataStorage.treasure` holds
  `4294967295` and `dataStorage.scenario` `4293655417` (u32 values), and
  `timeStampTicks` reaches 18 digits [M]. An invented value such as
  `639083329294237651` is above 2^53, so a parser that maps each number
  to an f64 (JavaScript's `JSON.parse`, for one) rounds it. FF III and
  FF IV saves hold no integer above i32. FF V and FF VI each hold a
  `timeStampTicks` above 2^53, and FF VI also a `scenario` above i32 [M].
- Keep the original text of each number you do not edit
  ([Principles](../architecture.md#principles), principle 1).
- Rust's `serde_json` 1.0.154 with `preserve_order` and
  `arbitrary_precision` re-serializes all 5,527 JSON documents of the 34
  files, at every nesting depth from 0 to 7, byte for byte, and
  re-escapes all 7,392 string tokens byte for byte [M]. Without
  `arbitrary_precision`, 4 `userData` documents break on
  `userData.playTime`; without `preserve_order` as well, 1,266 documents
  break on key order [M].

### Key order and spellings

Keep the key order you read. Keys shared by the six games appear in
one relative order, and no object repeats a key [M]. The game's
misspellings are the real key names [M]:

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

### Keys that naive editors drop

- The Go editor rebuilds each document from Go structs, so a key missing
  from a struct vanishes on save [C].
- `mapData.minimapEnable` exists in all six FF I and FF II slots [M].
  The Go struct had no field for it until `6ef3bc7`, and the minimap
  setting reset on each save [C].
- After the fixes, a round trip of the measured FF I and FF II saves
  loses no key [C, test in `6ef3bc7`]. Of all documents, only `mapData`
  keeps keys the structs do not declare [C]. Five keys that FF III to VI
  saves hold have no struct field and vanish on a round trip:
  `userData.winCount`, `braveBladeReliefCount`,
  `braveBladeEscapeCount` and `isTakeOverBraveBladeEscapeCount` (FF V),
  and `cheatSettingsData.jobProficiencyRate` (FF III) [M for presence, C
  for the struct].
- The structs declare optional keys absent from FF I and FF II, and the
  Go editor writes them only when the file had them [C]. FF III to VI
  saves hold these, per game [M]:

| Document | Extra keys | Games | Declared in the Go structs |
|---|---|---|---|
| `mapData` | `viewType` | III, V, VI | yes |
| `mapData` | `currentSelectedPartyId`, `otherPartyDataList`, `partyPlayableCharacterCorpsId`, `fieldDefenseNpcEntityIDList`, `beastFieldEncountExchangeFlags`, `beastFieldEncountSeekGroupId`, `rtsData` | VI | yes |
| `gpsData` | `transportationId` | III, IV, VI | yes |
| `userData` | `wonderWandIndex` | V | yes |
| `userData` | `winCount`, `braveBladeReliefCount`, `braveBladeEscapeCount`, `isTakeOverBraveBladeEscapeCount` | V | no |
| `cheatSettingsData` | `jobProficiencyRate` | III | no (9-field struct) |

- The structs also declare `configData.brightness`, `masterVolume`,
  `bgmVolume` and `seVolume`; no measured slot holds them [M, absence].

### Documents of the other files

| File | Keys, in order [M] |
|---|---|
| Bestiary | `scenarioFlags` (string, JSON), `monsterDefeats` (string, JSON), `totalSubjugationCount` (int) |
| Slot index | `keys` (ints 1 to 22), `values` (booleans) |
| System settings | `language`, `fontType`, `gamePadKeySettings`, `keyboardKeySettings`, `mouseKeySettings`, `screenMode`, `resolutionWidth`, `resolutionHight`, `isLeftActionIcon`, `isLeftVirtualPad`, `isLeftMenuCommand`, `isLeftBattleCommand`, `isAnalogMode`, `brightness`, `masterVolume`, `bgmVolume`, `seVolume`, `bgmType` |

The three `…KeySettings` values are dictionaries whose values are more
dictionaries in strings [M].

## Domain sections

Paths below use `.` for an object member and `→` for a step into a JSON
string.

### ID spaces and tables

A save points into the game data through three ID spaces: `contentId`,
`abilityId` and `jobId`. The Go editor names them in tables of its package
`models/core/costs/ff1`, and NeoFFSE takes the FF I names and IDs from
those tables ([Sources](#sources)). Facts about the tables are [C];
comparisons with the saves are [M].

| Space | Referenced by | Tables | IDs |
|---|---|---|---|
| `contentId` | rows of `normalOwnedItemList` and `importantOwendItemList`, equipment slots, the `contentId` of an ability object | `Items`, `ImportantItems`, `Weapons`, `Shields`, `Armors`, `Helmets`, `Gloves` | 2 to 202; spells 213 to 276 |
| `abilityId` | `abilityList`, spell slots, `learnedAbilityList` | `Abilities` | 5 to 94 |
| `jobId` | character `jobId`, `id` in `jobList`, `releasedJobs` | `Jobs` | 1 to 12 |

Items, key items and equipment share one `contentId` space. The seven
tables hold 116 entries over 115 distinct IDs, and no ID appears in two
tables; the one collision, 165, sits inside `Shields`. Key items take 45
to 61 [C], and every ID of `importantOwendItemList` falls in that range
[M]. Spell content IDs (`abilityId + 208` in FF I) sit above every table
ID, and none appears in the inventory [M].

Each table is a package-level slice of `models.NameValue` (`Name string`,
`Value int`) in `models/core/costs/ff1`, sorted by ID. The package imports
no GUI code. Its module path is `pixel-remastered-save-editor`.

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

- **Names.** All names are English ASCII, without `"` or `\`. Names
  repeat: "Wizard's Staff" at 122 and 123, "Gaze" six times, "Blaze" five
  times and "Icestorm" twice in `Abilities`, place names in `Maps` and boss
  variants in `Bestiary`. Look an entry up by ID, never by name.
- **Duplicate ID.** `Shields` lists 165 twice, as Ice Shield and Flame
  Shield. The Go editor merges `Items` and the five equipment tables into
  one ID-to-name map, and the later entry replaces the earlier one
  ([Open questions](#open-questions)).
- **Placeholders.** The first row of each equipment table is a
  placeholder: `[Empty R. Hand]` (62), `[Empty L. Hand]` (130), `[Empty
  Body]` (131), `[Empty Head]` (132) and `[Empty Arms]` (133). They are
  content IDs the game uses for an empty slot, not items ([Equipment
  slots](#equipment-slots)).
- **Spells and monster abilities.** In `Abilities`, 5 to 68 are 64 spells,
  8 levels of 8, and 69 to 94 are monster abilities that no character
  holds. Only the names mark the split ([Abilities](#abilities)).
- **Gaps** inside the ranges, placeholders aside. `Items` skips 4, 6, 9,
  16, 20 to 31 and 33. `Weapons` skips 66 to 70, 84, 85, 88, 90, 91, 93
  to 95, 98 to 100, 103 to 105, 108, 111 to 113 and 117. `Armors` skips
  135, 138 to 144, 151, 152, 155 and 159. `Shields` skips 166, 167, 169,
  171 and 172. `Helmets` skips 176 to 182. `Gloves` skips 196 and 199 to
  201. No table holds 124 to 129, 174, 189 or 190.
- **Comments.** The source comments out X-Potion, six daggers and Knight's
  Armor. They have no ID in the tables.
- **IDs the tables do not name.** The saves hold content IDs 67, 100, 103,
  151 and 166, and command 5, that the source tables lack. The content IDs
  sit inside the ranges of `Weapons` (67, 100, 103), `Armors` (151) and
  `Shields` (166). All five appear in the inventory of slot 1, and four of
  them in equipment ([Equipment slots](#equipment-slots)) [M].

### Characters

`userData → ownedCharacterList → target[]` holds one JSON string per
character: 4 in FF I, 7 or 8 in FF II [M]. Identify a character by
`id`; names repeat and change.

| Key | Type | Notes |
|---|---|---|
| `id` | int | 1 to N, unique [M] |
| `characterStatusId` | int | FF I 1 to 6, FF II 1 to 10 [M]. FF I: `jobId` = `characterStatusId` + 6 for 4 of 4 promoted characters [M]; the status names the base class [I]. No other meaning measured |
| `isEnableCorps` | bool | 4 of 8 FF II characters are `false` [M]; the Go editor runs its per-game ability rules only for `true` [C] |
| `jobId` | int | FF I 7 to 12, FF II 1 to 9 [M]; FF I measured 7, 8, 11 and 12, all promoted classes; IDs in `Jobs` ([ID spaces and tables](#id-spaces-and-tables)) |
| `name` | string | raw UTF-8, may hold non-ASCII [M]; two FF I heroes may share a name, FF VI has two Mogs [C, `d8a6c6f`] |
| `currentExp` | int | [M] |
| `parameter` | JSON | stats, below |
| `commandList` | JSON list of ints | FF I `[1,3,4,2,5]`, FF II `[1,3,4,2,5,7]` [M]; the Go editor writes it only when the user enabled command editing [C]. FF I value 5 has no entry in the editor's `Commands` table (1 to 4) [M, C] |
| `abilityList`, `abilitySlotDataList`, `additionOrderOwnedAbilityIds`, `sortOrderOwnedAbilityIds`, `abilityDictionary`, `skillLevelTargets`, `learningAbilitys` | JSON | [Abilities](#abilities) |
| `jobList` | JSON list of `{id, level, currentProficiency}` | FF I 12 entries (`id` 1 to 12), FF II 8 [M]. FF I: `level` 1 and `currentProficiency` 0 in every entry of the 8 character states [M]; FF I does not use them [I] |
| `equipmentList` | JSON dictionary | keys 1 to 5 (equipment slots), values `{contentId, count}` [M]; `count` above 1 occurs (FF I up to 7, FF II up to 4); slots and `count` in [Equipment slots](#equipment-slots) |
| `equipmentAbilitys` | JSON list | `[0,0,0]` in FF I and FF II [M] |
| `numberOfButtles`, `ownedMonsterId`, `magicStoneId`, `magicLearningValue` | int | `0` in FF I [M]; `magicStoneId` is the FF VI equipped esper [C] |

`parameter` keys [M]: `currentHP`, `currentMP`, `currentMpCountList`,
`addtionalMaxMpCountList` (dictionaries with 8 keys, one per spell level,
in FF I and FF II), `addtionalLevel`, `addtionalMaxHp`, `addtionalMaxMp`,
`addtionalPower`, `addtionalVitality`, `addtionalAgility`,
`addionalWeight`, `addtionalIntelligence`, `addtionalSpirit`,
`addtionalAttack`, `addtionalDefense`, `addtionalAbilityDefense`,
`addtionalAbilityEvasionRate`, `addtionalMagic`, `addtionalLuck`,
`addtionalAccuracyRate`, `addtionalEvasionRate`,
`addtionalAbilityDisturbedRate`, `addtionalCriticalRate`,
`addtionalDamageDirmeter`, `addtionalAbilityDefenseRate`,
`addtionalAccuracyCount`, `addtionalEvasionCount`,
`addtionalDefenseCount`, `addtionalMagicDefenseCount`,
`currentConditionList` (`{"target":[]}` in all 35 measured characters).

**FF I `parameter` fields.** Several `addtional…` names hold a gain on top
of a base the file does not store:

- `addtionalLevel` grows with `currentExp` in both game states [M]. The Go
  editor labels it "Level" (`coreStats.go`) [C]. Whether the menu shows
  the value or the value plus 1 is open ([Open questions](#open-questions)).
- `addtionalMaxHp` is a gain over a per-character base, not the maximum.
  `currentHP` exceeds it in 7 of 8 character states, by a difference that
  stays constant per character across both game states [M]. In the
  eighth state `currentHP` is 999 and `addtionalMaxHp` is 1 097; the same
  character's difference in the other game state is 35, so base plus gain
  would be 1 132. Reading: maximum HP = base + `addtionalMaxHp`, capped at
  999 [I]. The Go editor labels the field "HP Max" (`coreStats.go`) [C],
  which the data contradicts.
- `currentMP` and `addtionalMaxMp` hold 0 in all 8 states. FF I stores
  spell charges per spell level in `currentMpCountList` and
  `addtionalMaxMpCountList` (keys 1 to 8, values 0 to 9) [M].
  `addtionalMaxMpCountList` is a gain too: for the two spellcasters, level
  1 shows `currentMpCountList` 9 against `addtionalMaxMpCountList` 7, and
  every other level shows equal values [M]. Reading: maximum charges =
  base + `addtionalMaxMpCountList`, capped at 9 [I].

### Equipment slots

FF I characters hold five equipment slots, keys 1 to 5 of `equipmentList`
in that order. An empty slot holds the placeholder `contentId` of its slot,
not 0.

| Key | Slot (Go editor label) | Table | Placeholder |
|---|---|---|---|
| 1 | R. Hand | `Weapons` | 62 |
| 2 | L. Hand | `Shields` | 130 |
| 3 | Body | `Armors` | 131 |
| 4 | Head | `Helmets` | 132 |
| 5 | Arms | `Gloves` | 133 |

- **Keys and slots.** The keys read `[1,2,3,4,5]` in that order for every
  FF I character [M]. The Go editor labels the rows by position in
  `values` (`coreEquipment.go`) [C]. 34 of 40 slot entries (slot 1 and
  Quick Save, 20 each) resolve to the table of their slot [M]. The other
  6 hold IDs that no table names, each inside the ID range of its slot's
  table [M]. FF I has no accessory slot.
- **Empty slots.** One slot entry holds a placeholder: 130 in a left hand
  of the Quick Save [M, 1 entry]. The tables list 62, 131, 132 and 133 as
  the placeholders of the other slots [C]; no sample holds one in a slot.
- **`count` copies the inventory count.** In FF I, `count` equals the
  inventory count of the same `contentId` in 40 of 40 slot entries,
  placeholder 130 included (count 9, inventory 9) [M]. In FF II, 54 of 75
  entries match and all 21 others sit below the inventory count [M].
  Reading: the game copies the inventory count when a character equips
  and does not refresh it [I]. Whether the game reads `count` at all is
  open.

### Abilities

An ability carries two IDs. `abilityId` points into the game's ability
table; `contentId` points into the content table that items and
equipment share [I]. Each game links the two by a fixed offset for its
spells, with exceptions, and a writer must fill both.

Ability object: `{"abilityId":int,"contentId":int,"skillLevel":int}`.

| Game | `contentId − abilityId` | Tag |
|---|---|---|
| FF I | 208 | [M] 47 of 47 abilities and slot spells, slot 1 and Quick Save |
| FF II | 207 for spells; ability 711 maps to content 937 (offset 226) | [M] 35 spells at 207; each FF II character holds ability 711 |
| FF III | 201 | [C] |
| FF IV | 310 | [C] |
| FF V | 266 | [C] |
| FF VI | 330 | [C] |

The Go editor applies the offset only to an ability whose `contentId`
is 0 [C]. Ability 711 is missing from its FF II table, which stops at
644 [C].

Character keys that reference abilities:

| Key | Holds | Rule |
|---|---|---|
| `abilityList` | ability objects as JSON strings | one entry per `abilityId` [M] |
| `additionOrderOwnedAbilityIds` | content IDs | the content IDs of `abilityList`, same order, in all 35 characters of the six slots [M] |
| `sortOrderOwnedAbilityIds` | content IDs | empty or a subset of the owned ones; only FF II had entries [M]; the Go editor never edits it |
| `abilityDictionary` | dictionary | FF II: key `3` maps to `{"target":[ability JSON strings]}`; FF I: empty [M]; meaning not measured |
| `skillLevelTargets` | dictionary | FF II: keys 0 to 9 with ints; FF I: empty [M]; meaning not measured |
| `learningAbilitys` | list | empty in FF I and FF II [M] |

`userData → learnedAbilityList` holds ability IDs. In FF I it equals the
union of the party's `abilityId`s (47 IDs) [M]. In FF II it does not, and
its meaning is unknown [M].

**FF I spell slots** [M]. `abilitySlotDataList → target[]` holds 8 JSON
strings, one per spell level: `{"level":1..8,"slotInfo":"<JSON>"}`.
`slotInfo` is `{"keys":[1,2,3],"values":[v1,v2,v3]}`, each value either
`""` (empty slot) or an ability JSON string. In both FF I files, each
spell of `abilityList` sits in a slot and each slot spell sits in
`abilityList`.

FF I writer rule [C, `375f916`; consistent with M]: for each spell in a
slot, set `contentId = abilityId + 208`, list it once in `abilityList`,
and list its content ID once in `additionOrderOwnedAbilityIds`.

**FF I spell level and school** [M, 47 of 47 slot spells]. `abilityId`
fixes both. The two spellcasters hold 47 spells in all, `abilityId` 5 to
68, every one in `Abilities`.

- Level = (`abilityId` − 5) / 8 + 1, with integer division. It equals the
  `level` of the `abilitySlotDataList` entry that holds the spell.
- In each level, offsets 0 to 3 (`(abilityId − 5) % 8`) hold the White
  Wizard's spells (24 of 24) and offsets 4 to 7 the Black Wizard's (23 of
  23). The names agree: 5 to 8 are Cure, Protect, Dia and Blink, and 9 to
  12 are Fire, Focus, Sleep and Thunder [C].
- Knight and Ninja hold no spell, so the corpus does not test their access
  to white or black magic.
- `Abilities` lists 5 to 68 as spells and 69 to 94 as monster abilities
  [C]; no character holds an ID above 68 [M].

**`skillLevel`.** FF I: `0` [M]. FF II: spell proficiency, 0 to 456
measured; the Go editor edits it as a level [M, C]. FF VI: learning
percentage, 100 means learned [C].

### Party

`userData → corpsList` lists the active party; keep `characterId` values
pointing at existing character `id`s.

- `corpsList → target[]`: 4 JSON strings `{"id":1,"characterId":n}` in
  FF I, FF II, FF III, FF V and FF VI, 5 in FF IV; `id` is 1 in each
  entry in FF I and FF II [M].
- `corpsSlots`: dictionary with keys `0`, `1`, `2`; each value is a list
  with the same entry shape [M]. In the six slots all three lists equal
  `corpsList` [M]. FF VI uses several parties [C].
- `userData.corpsSlotIndex`: `0` in all six slots [M].
- `mapData.playableCharacterCorpsId`: 0 to 3 [M]; meaning not measured.
- `userData → releasedJobs → target`: FF I 1 to 12, FF II 4 entries [M].

### Inventory

`userData → normalOwnedItemList → target[]` holds `{contentId, count}`
JSON strings in the game's own order; the sort list holds content IDs,
never row numbers.

- Normal items: 47 to 56 entries, no duplicate `contentId`, no `count`
  of 0, not sorted by ID [M].
- `importantOwendItemList`: same shape, `count` 1 [M]. In FF I it holds 9
  or 10 entries, IDs 45 to 61, all in `ImportantItems` [M].
- **The inventory includes equipped items.** In FF I, each equipped
  `contentId` appears in `normalOwnedItemList` with a count at least the
  number of characters who equip it. A ring worn by all 4 characters has
  count 7 in slot 1 and 6 in the Quick Save [M]. No equipped ID is
  missing or under-counted [M].
- **The inventory holds the five placeholders.** In FF I, 62, 130, 131, 132
  and 133 sit in the list in both game states, as its first five rows, with
  counts 5, 9, 5, 9 and 9 [M]. What those counts mean is unknown.
- **IDs the tables do not name.** The FF I list holds 5 IDs (4 in the Quick
  Save) that no source table names: 67, 100, 103, 151 and 166 ([ID spaces
  and tables](#id-spaces-and-tables)) [M]. Keep them as read.
- `normalOwnedItemSortIdList → target`: content IDs. FF II slot 1 and
  FF II Auto Save hold 23, each one an owned item; the other four slots
  hold none [M]. The Go editor's "Reset Sort" wrote `1..N` with empty rows
  until `6ef3bc7` [C].
- The Go editor drops rows with `contentId` 0 and keeps the sort list
  unless the user resets it [C].
- `warehouseItemList`, `ownedMagicList`: empty in FF I and FF II [M].
- `ownedKeyWaordList → target`: FF II keywords, ints 917 to 931 [M];
  content IDs [I].

### Party position and map

`mapData` holds the player's map, point and position. A writer copies
each key back, including keys it does not edit.

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

The Go editor never copied `mapId`, `pointIn`, `transportationId` and
`carryingHoverShip` back, so edits to them reverted on save until
`6ef3bc7` [C]. [FF VI](#ff-vi) lists the FF VI keys.

### Transportation

`userData → ownedTransportationList → target[]` holds one JSON string
per vehicle: 4 in FF I, 7 in FF II [M].

- Keys: `position` (`x`, `y`, `z` floats), `direction` (0 to 8), `id`,
  `mapId`, `enable`, `timeStampTicks` [M].
- An unplaced vehicle has `mapId` −1 and a position such as
  `(1000.0, 1000.0, -1000.0)` [M].
- `timeStampTicks` counts .NET DateTime ticks: 100 ns units since
  0001-01-01. The invented value `639083329294237651` reads 2026-03-05
  18:42:09; the measured values fall in the date range of the saves
  [M, I]. `0` means never set [M].
  Store it as i64. The time zone (local or UTC) is unknown.
- The Go editor's `global.NowToTicks` adds `621259200000000000`
  (`86400 × 365 × 1970 × 10^7`), 112 days short of the true Unix-epoch
  offset `621355968000000000` [C, arithmetic]. No code calls it [C].

### Counters and flags

| Key in `userData` | Type | Notes |
|---|---|---|
| `owendGil` | int | up to 999 999 measured |
| `totalGil` | int | `0` in all six slots |
| `playTime` | float | seconds; an invented example is `13621.814333377138` |
| `steps`, `escapeCount` | int | `0` in all six slots |
| `battleCount`, `openChestCount`, `saveCompleteCount`, `monstersKilledCount` | int | |
| `currentArea`, `currentLocation` | string | free text; `currentLocation` may be `""` |
| `owendCrystalFlags` | JSON list | 4 booleans in FF I and FF II |
| `configData` | string | `""` in all 14 game slots |
| `cheatSettingsData` | object | 9 keys: `isEnableEncount`, `expRate`, `gilRate`, `learningProficiencyRate`, `isHpIncreaseByFightCount`, `abpRate`, `magicProficiencyRate`, `parameterUpRate`, `magicLearningProficiencyRate`; FF III adds `jobProficiencyRate` |
| `isOpenedGameBoosterWindow` | bool | |

All rows [M].

### Time stamp and progress flags

`timeStamp` is a formatted local date whose layout varies by game; keep
the string as read and write an edit in the same layout.

| Files | Layout | Invented example |
|---|---|---|
| FF I, all three slots | `M/d/yyyy h:mm:ss tt` [M] | `3/17/2026 6:42:09 PM` |
| FF II, all three slots | `dd/MM/yyyy HH:mm:ss` [M] | `17/03/2026 18:51:30` |
| FF III to VI, slot 1 and Auto Save | the FF I layout [M] | |

- FF II is the only game with the `dd/MM/yyyy` layout. FF I and FF II
  ran on the same machine on the same days and still used different
  layouts [M]. The Go code says the game uses the system
  culture [C]; the measurement leaves the cause open
  ([Open questions](#open-questions)).
- The Go editor also parses `M/d/yyyy HH:mm:ss`, `dd.MM.yyyy
  HH:mm:ss`, `yyyy/MM/dd H:mm:ss`, `yyyy-M-d H:mm:ss` and ISO 8601 [C].
- Until `6ef3bc7` the Go editor prefixed `0` to a date like
  `1/2/2026 …`, parsed with the single layout `01/02/2006 15:04:05 PM`,
  and rewrote the stamp on save [C].

`dataStorage` keys [M]: `scenario` (128 ints), `treasure` (128 ints),
`global` (16 ints), `area` (8), `map` (8), `selected`, `itemSelected`,
`transportation` (empty). `scenario` and `treasure` hold u32 values up
to `4294967295`; the values look like bit sets [I]. FF VI stores the Cursed
Shield fight count in `global[9]` [C].

### Bestiary

The bestiary file serves every slot of a game. `monsterDefeats` maps
monster IDs to defeat counts in first-defeat order, and
`totalSubjugationCount` equals the sum of the counts.

- `scenarioFlags → target`: 1024 ints, each 0 or 1; FF I has 85 ones,
  FF II 83 [M]. Meaning not measured; the Go editor never edits them.
- `monsterDefeats`: `{"keys":[ids],"values":[counts]}`, keys unique,
  counts ≥ 1, keys unsorted [M].
- `totalSubjugationCount` = sum of `values`: 3463 in FF I, 2977 in FF II
  [M].
- It differs from a slot's `userData.monstersKilledCount` (FF I slot 1:
  3458) [M], since one bestiary spans all slots [I].
- A complete FF I bestiary holds IDs 1 to 128, each once [M]. The FF II
  file holds 75 IDs, the highest 118 [M].
- The ID tables list boss variants above 128 (FF I 129, FF II 129 to
  142); the game counts them under the base monster, and no measured
  file holds an ID above 128 [C, M].

"Defeat all" rules of the Go editor [C, `models/core/bestiary.go`]:

1. Keep each existing entry and its count.
2. Append each missing ID with count 1, in ascending ID order, after the
   existing entries.
3. Skip IDs ≤ 0 and, in FF I and FF II, IDs above 128.
4. Before writing, drop entries with ID ≤ 0 or count ≤ 0, merge
   duplicate IDs by adding their counts at the first position, and
   recompute `totalSubjugationCount`.

Write the bestiary only when it loaded and changed. Keep
`scenarioFlags`, unknown keys and number text as read [C].

### FF VI

The FF VI key names in `mapData` are measured [M]. The measured FF VI
saves come from early game: they hold no esper and no filled
`playerEntity` of another party. The rules below come from the Go code
[C].

- Espers: `userData → ownedMagicStoneList → target` lists owned esper
  IDs; a character's `magicStoneId` holds the equipped one.
- Magic: each spell sits in `abilityList` with `skillLevel` as the
  learning percentage; 100 means learned; `contentId = abilityId + 330`.
- Other parties: `mapData.otherPartyDataList` is a plain array of
  objects `{mapId, pointIn, playerEntity, playableCharacterCorpsId}`,
  with `playerEntity` a JSON string as in
  [Party position and map](#party-position-and-map). Related keys:
  `partyPlayableCharacterCorpsId`, `currentSelectedPartyId`.
- Cursed Shield fights: `dataStorage.global[9]`.

### Slot `configData`

`configData` keys, in order [M]: `buttonType`, `battleModeIndex`,
`battleSpeedIndex`, `battleMessageSpeedIndex`, `isCursorMemory`,
`isKeepAutoBattle`, `messageSpeed`, `isLeftActionIcon`,
`isLeftVirtualPad`, `isLeftMenuCommand`, `isLeftBattleCommand`,
`reequipIndex`, `analogModeIndex`, `isAutoDash` (an int, `1`). The keys
are the same in all six games. No slot carries volume or brightness
keys, and the Go struct's optional ones never occur; those settings live
in the system settings file [M].

## Write rules

A writer changes the keys the user edited and nothing else, re-encodes
the container as in [Pseudo-code](#pseudo-code), and goes through the
safe write pipeline of [Save safety](../architecture.md#save-safety).

1. Write the BOM back when the file had one [C]. Write back the trailer
   of the decoded container. The game ends its files with CRLF and also
   loads a file without it [G].
2. Terminate the DEFLATE stream with a final block, pad with zeros to a
   multiple of 32, and use the key and IV of [Cipher](#cipher).
3. Re-serialize only the string documents that hold an edit, and their
   parent strings up to the slot. Leave every other string byte for
   byte as read.
4. Serialize in the game's style: compact, raw UTF-8, floats with a
   decimal point, original key order and spellings. Go's encoder escapes
   `<`, `>` and `&` as `\u003c`, `\u003e` and `\u0026` [M]; the game
   wrote no `\u` escape in the measured files.
5. Set `id` to the slot number of the target file
   ([Slot files](#slot-files)) when you write
   a slot's data under another slot's name [M, two editor files].
6. Leave the slot index as read when you write into a slot with no file:
   the load menu lists a slot file whose flag is `false` ([Slot
   2](#slot-2-of-ff-i-and-ff-ii)) [G, FF I and FF II].
7. Fill both ability IDs ([Abilities](#abilities)) and keep the FF I lists consistent.
8. Keep `timeStamp` as read unless the user edited it; write an edit in
   the same layout [C].
9. Write the bestiary only when it loaded and changed; leave an
   unreadable bestiary untouched [C].
10. The Go editor writes the slot, then the bestiary; a failure between
    the two leaves a new slot with the old bestiary [C].
11. Back up every file you replace before the first write. The Go
    editor copies them to `<config dir>/backups/FF<n>/<YYYYMMDD-HHMMSS>/`,
    next to its own config file, with a `backup.json` manifest, and keeps
    20 backups per game [C].
    [Save safety](../architecture.md#save-safety) gives NeoFFSE's layout.
12. Steam Cloud may report a conflict on the next launch; the player
    must keep the local files ([Save safety](../architecture.md#save-safety)).

## Known pitfalls

One line per bug the Go editor had or the format invites. The tag
gives the source; a commit ID points to the fix.

1. `rijndael.Decrypt` deletes stream zero bytes with the padding; inflate fails (FF II Quick Save, slot index of FF I and FF II; no FF III to VI file) [M, `6f8f74a`].
2. Files the Go editor's encoder writes can end in a zero byte that pitfall 1 deletes [M, two editor files, `6f8f74a`].
3. The Go editor's loader ignored base64 errors and carried on with empty data [C, `6f8f74a`].
4. A bestiary "eof" error-string hack dereferenced a nil error when the bestiary decoded fine [C, `6f8f74a`].
5. An AES library cannot decrypt the files: the block is 256 bits ([Cipher](#cipher)).
6. The library's `Encrypt` pads in place and can write into the caller's buffer [C, `codec.go`].
7. Struct-based re-serialization drops unknown keys; `minimapEnable` reset on each save [C, `6ef3bc7`].
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
21. The bestiary was rewritten and backed up on each save, even unchanged [C, `6f8f74a`, `68e6a80`].
22. Character lookup by name opened the wrong character; use `id` [C, `d8a6c6f`].
23. Switching between PC and PlayStation kept file names and the codec of the other mode [C, `7a83b22`].
24. A switch to an empty folder kept slot buttons bound to the old folder, and "save" overwrote the real game save [C, `7a83b22`].
25. A crash during a non-atomic write left an empty or partial slot [C, `375f916`].
26. An empty nested document is `""`; writing `"{}"` changes the file [M].
27. The Go editor labels `addtionalMaxHp` "HP Max", but the field is a gain over a base ([Characters](#characters)) [M, C].
28. The `Shields` table lists ID 165 twice, and the editor's ID-to-name map keeps the later entry [C].

## Open questions

The measured files leave these points open. Each needs a real save or
an in-game test.

1. FF III to VI: slot names 2 to 20 and the Quick Save name (FF I and
   FF II: slot names 3 to 20), the ability offsets, FF VI espers and
   other parties, and how the game writes non-ASCII text in FF III, V
   and VI. The measured saves come from early game.
2. Does the game load a file without the BOM?
3. Does the game load a float written as an integer (`58` for `58.0`)?
   The Go editor writes that form [M]; no in-game test covers it.
4. Does the game pad an aligned stream with a full block of zeros, and
   does it accept zero bytes of padding? None of the 34 files had a
   stream length divisible by 32.
5. The `timeStamp` layout: FF I and FF II used different layouts on one
   machine, and the setting that picks the layout is unknown.
6. `timeStampTicks`: local time or UTC?
7. Meaning of `clearFlag` against `isCompleteFlag` (the FF I Auto Save
   holds `clearFlag` 0 and `isCompleteFlag` 1), of the top-level
   `playTime` (`0.0` in all 14 game slots), of `characterStatusId`, `pointIn`,
   `playableCharacterCorpsId`, `skillLevelTargets`, `abilityDictionary`,
   `scenarioFlags`, and the FF II `learnedAbilityList`.
8. Does the game accept an inventory item missing from a non-empty
   `normalOwnedItemSortIdList`?
9. The PlayStation variant: file layout after extraction, presence of
   a bestiary, BOM.
10. System settings `language`: value `3` in all six games; mapping not
    measured.
11. Which name belongs to the `Shields` ID 165, Ice Shield or Flame
    Shield. A save holds 166, the free ID next to it, in a left-hand slot,
    so 166 may belong to one of the two [I]. An in-game look at that slot
    would settle it.
12. Three in-game readings for one FF I character. The menu level for
    `addtionalLevel` 69: 69 or 70. The maximum HP when base plus
    `addtionalMaxHp` reaches 1 132: 999 or 1 132. The maximum charges at
    spell level 1, where `currentMpCountList` is 9 and
    `addtionalMaxMpCountList` is 7.
13. Does the game read equipment `count`, and what do the inventory counts
    5 and 9 of the placeholders mean?
14. An FF I save from before the class change: which `jobId` does a
    character hold (1 to 6, if `characterStatusId` names the base class),
    and do the placeholders 62, 131, 132 and 133 fill bare slots? No
    measured file comes from a new game.
