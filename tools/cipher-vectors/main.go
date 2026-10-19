// SPDX-License-Identifier: MIT OR Apache-2.0

// Command cipher-vectors writes Rijndael-256 test vectors for the Pixel
// Remaster save cipher. It drives KiameV's ffpr-save-cypher library, the
// reference the Rust port in neoffse-format-pr must reproduce.
//
// Run it from this directory:
//
//	go run . -o ../../crates/neoffse-format-pr/tests/data/rijndael256-vectors.txt
//
// Without -o the vectors go to stdout. The output depends on the pinned
// library version and on the seed below and on nothing else, so two runs
// give byte-identical files. Use -o on Windows: a PowerShell redirect
// would re-encode the file.
package main

import (
	"bytes"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"runtime/debug"
	"strings"

	"github.com/kiamev/ffpr-save-cypher/rijndael"
)

const (
	blockSize  = 32
	modulePath = "github.com/kiamev/ffpr-save-cypher"
	moduleVer  = "v1.0.0"

	// seed is "NeoFFSE" in ASCII. Changing it, or the order of the
	// sections, changes every random vector.
	seed = 0x4e656f46465345

	// keyHex is the PR key from pixel-remaster.md#cipher. The library keeps
	// its key private; this copy only gives the block section a plaintext
	// the Rust port also knows. A typo here changes one vector, not the
	// cipher.
	keyHex = "61090700b9c3b8b9421599e79ca57b87bea932d3797bad7663ed4dde0a943cc5"

	// ivHex is the PR IV from pixel-remaster.md#cipher. The library fixes
	// its IV inside New; checkIV compares this copy with the one the
	// library applies, so the document and the generator cannot drift.
	ivHex = "0224f9771fdb6e0e3bd508d7b795bf2e0cbd176942680a637b12bc6273db2ebb"
)

// header is the top of the data file. It carries the grammar, so a reader
// understands the file without the repository.
const header = `# Rijndael-256 test vectors for the Pixel Remaster save cipher.
# Generator: tools/cipher-vectors, driving %s %s (MIT, KiameV).
# Regenerate, from tools/cipher-vectors:
#   go run . -o ../../crates/neoffse-format-pr/tests/data/rijndael256-vectors.txt
# Random bytes: splitmix64, seed 0x%x.
#
# One record per line. Fields are separated by one space. Hex is lowercase.
# Lines that start with # and blank lines carry no data.
#
#   block <plain> <cipher>
#       One 32-byte block through the raw cipher: no IV, no chaining, no
#       padding. Check both directions.
#   encrypt <plain> <cipher>
#       CBC encryption with the PR key and IV. <plain> has one byte or more
#       and may be unaligned. The encryptor appends zero bytes up to a
#       multiple of 32 (none when aligned); <cipher> has the padded length.
#   decrypt <returned> <cipher> <plain>
#       CBC decryption with the PR key and IV. <plain> has the length of
#       <cipher> and is never stripped. <returned> is the decimal length
#       that the Go library's Decrypt returned: it cuts up to 31 trailing
#       zero bytes, see pixel-remaster.md#zero-padding-and-the-stripping-trap.
`

// prng is splitmix64. A local generator ties the vector bytes to this file
// alone, not to the standard library of the Go release that runs it.
type prng struct{ state uint64 }

func (p *prng) next() uint64 {
	p.state += 0x9e3779b97f4a7c15
	z := p.state
	z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
	z = (z ^ (z >> 27)) * 0x94d049bb133111eb
	return z ^ (z >> 31)
}

func (p *prng) bytes(n int) []byte {
	out := make([]byte, 0, n+8)
	for len(out) < n {
		out = binary.LittleEndian.AppendUint64(out, p.next())
	}
	return out[:n]
}

// nonzero returns n bytes, none of them zero. A zero would blur where the
// data ends and the padding begins in the stripping cases.
func (p *prng) nonzero(n int) []byte {
	out := p.bytes(n)
	for i, b := range out {
		if b == 0 {
			out[i] = 1
		}
	}
	return out
}

type sample struct {
	label string
	data  []byte
}

type gen struct {
	cipher rijndael.Rijndael
	rng    prng
	out    strings.Builder
}

func main() {
	path := flag.String("o", "", "write the vectors to this file instead of stdout")
	flag.Parse()
	if err := run(*path); err != nil {
		fmt.Fprintln(os.Stderr, "cipher-vectors:", err)
		os.Exit(1)
	}
}

func run(path string) error {
	if err := checkModule(); err != nil {
		return err
	}
	g := &gen{cipher: rijndael.New(), rng: prng{state: seed}}
	if err := g.checkIV(); err != nil {
		return err
	}
	fmt.Fprintf(&g.out, header, modulePath, moduleVer, seed)
	for _, section := range []func() error{g.blocks, g.encrypts, g.decrypts} {
		g.out.WriteString("\n")
		if err := section(); err != nil {
			return err
		}
	}
	if path == "" {
		_, err := os.Stdout.WriteString(g.out.String())
		return err
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	return os.WriteFile(path, []byte(g.out.String()), 0o644)
}

// checkModule fails when the build resolved another library version than the
// one named in the file header. go.mod pins it; this guards a stray replace
// directive or a go.work file.
func checkModule() error {
	info, ok := debug.ReadBuildInfo()
	if !ok {
		return errors.New("no build information: build with module support")
	}
	for _, dep := range info.Deps {
		if dep.Path != modulePath {
			continue
		}
		if dep.Replace != nil || dep.Version != moduleVer {
			return fmt.Errorf("%s resolved to %s, want %s", modulePath, dep.Version, moduleVer)
		}
		return nil
	}
	return fmt.Errorf("%s is not part of the build", modulePath)
}

// checkIV fails when the library's CBC path starts from another IV than
// ivHex. CBC encrypts an all-zero first block to E(0 xor IV) = E(IV), so
// that ciphertext must equal the raw block cipher applied to ivHex. The raw
// cipher comes from encryptBlock, which does not depend on the IV.
func (g *gen) checkIV() error {
	iv, err := hex.DecodeString(ivHex)
	if err != nil {
		return err
	}
	if len(iv) != blockSize {
		return fmt.Errorf("iv has %d bytes, want %d", len(iv), blockSize)
	}
	first, err := g.encrypt(make([]byte, blockSize))
	if err != nil {
		return err
	}
	raw, err := g.encryptBlock(iv)
	if err != nil {
		return err
	}
	if !bytes.Equal(first, raw) {
		return errors.New("the library's CBC IV differs from ivHex (pixel-remaster.md#cipher)")
	}
	return nil
}

func (g *gen) note(format string, args ...any) {
	fmt.Fprintf(&g.out, "# %s\n", fmt.Sprintf(format, args...))
}

func (g *gen) blocks() error {
	key, err := hex.DecodeString(keyHex)
	if err != nil {
		return err
	}
	if len(key) != blockSize {
		return fmt.Errorf("key has %d bytes, want %d", len(key), blockSize)
	}
	counting := make([]byte, blockSize)
	for i := range counting {
		counting[i] = byte(i)
	}
	firstBit := make([]byte, blockSize)
	firstBit[0] = 0x01
	lastBit := make([]byte, blockSize)
	lastBit[blockSize-1] = 0x80
	samples := []sample{
		{"zeros", make([]byte, blockSize)},
		{"the key itself", key},
		{"counting 0..31", counting},
		{"all 0xff", bytes.Repeat([]byte{0xff}, blockSize)},
		{"bit 0 of byte 0", firstBit},
		{"bit 7 of byte 31", lastBit},
	}
	for i := 0; i < 8; i++ {
		samples = append(samples, sample{fmt.Sprintf("seeded %d", i), g.rng.bytes(blockSize)})
	}
	for _, s := range samples {
		enc, err := g.encryptBlock(s.data)
		if err != nil {
			return err
		}
		dec, err := g.decryptBlock(enc)
		if err != nil {
			return err
		}
		if !bytes.Equal(dec, s.data) {
			return fmt.Errorf("block %q: decrypting the encryption does not give the block back", s.label)
		}
		g.note("block: %s", s.label)
		fmt.Fprintf(&g.out, "block %x %x\n", s.data, enc)
	}
	return nil
}

func (g *gen) encrypts() error {
	var samples []sample
	for _, n := range []int{1, 2, 31, 32, 33, 63, 64, 65, 96, 1000, 4096} {
		samples = append(samples, sample{"seeded", g.rng.bytes(n)})
	}
	samples = append(samples, sample{"all zero", make([]byte, 32)}, sample{"all zero", make([]byte, 33)})
	for _, s := range samples {
		ct, err := g.encrypt(s.data)
		if err != nil {
			return err
		}
		g.note("encrypt: %d bytes (%s), %d bytes of zero padding", len(s.data), s.label, len(ct)-len(s.data))
		fmt.Fprintf(&g.out, "encrypt %x %x\n", s.data, ct)
	}
	return nil
}

func (g *gen) decrypts() error {
	for _, blocks := range []int{1, 2, 3, 16} {
		ct := g.rng.bytes(blocks * blockSize)
		if err := g.decryptRecord(fmt.Sprintf("%d seeded blocks", blocks), ct); err != nil {
			return err
		}
	}
	cases := []struct{ length, zeros int }{
		{32, 0}, {32, 1}, {32, 31}, {32, 32},
		{64, 0}, {64, 31}, {64, 32}, {64, 40},
		{45, 0}, {45, 2},
	}
	for _, c := range cases {
		data := append(g.rng.nonzero(c.length-c.zeros), make([]byte, c.zeros)...)
		ct, err := g.encrypt(data)
		if err != nil {
			return err
		}
		label := fmt.Sprintf("%d data bytes ending in %d zero bytes", c.length, c.zeros)
		if err := g.decryptRecord(label, ct); err != nil {
			return err
		}
	}
	return nil
}

func (g *gen) decryptRecord(label string, ct []byte) error {
	plain, returned, err := g.decrypt(ct)
	if err != nil {
		return err
	}
	g.note("decrypt: %s; Decrypt returned %d of %d bytes", label, returned, len(ct))
	fmt.Fprintf(&g.out, "decrypt %d %x %x\n", returned, ct, plain)
	return nil
}

// encrypt runs the library's CBC encryption and checks the padding rule on
// the result length.
//
// Encrypt appends the zero padding to the slice it receives, and append
// writes into spare capacity of the caller's array. The clone keeps the
// caller's slice exactly as the caller wrote it
// (pixel-remaster.md#known-pitfalls, item 6).
func (g *gen) encrypt(plain []byte) ([]byte, error) {
	ct, err := g.cipher.Encrypt(bytes.Clone(plain))
	if err != nil {
		return nil, err
	}
	if want := (len(plain) + blockSize - 1) / blockSize * blockSize; len(ct) != want {
		return nil, fmt.Errorf("encrypt of %d bytes returned %d bytes, want %d", len(plain), len(ct), want)
	}
	return ct, nil
}

// decrypt returns the full CBC plaintext of ct and the length Decrypt
// reported.
//
// Decrypt cuts up to 31 trailing zero bytes (padder.Decode). The library
// only drops zeros, so appending that many zero bytes restores the
// plaintext. Two checks keep the claim honest: the cut length must follow
// the documented rule (pixel-remaster.md#zero-padding-and-the-stripping-trap),
// and encrypting the restored plaintext must give ct back. An aligned
// plaintext gets no padding from Encrypt, so the second check is exact.
func (g *gen) decrypt(ct []byte) (plain []byte, returned int, err error) {
	out, err := g.cipher.Decrypt(bytes.Clone(ct))
	if err != nil {
		return nil, 0, err
	}
	if len(out) > len(ct) {
		return nil, 0, fmt.Errorf("decrypt returned %d bytes for %d bytes of input", len(out), len(ct))
	}
	returned = len(out)
	plain = append(out, make([]byte, len(ct)-len(out))...)

	trailing := 0
	for trailing < len(plain) && plain[len(plain)-1-trailing] == 0 {
		trailing++
	}
	if want := len(ct) - min(trailing, blockSize-1); returned != want {
		return nil, 0, fmt.Errorf("decrypt returned %d bytes, the stripping rule predicts %d", returned, want)
	}
	again, err := g.encrypt(plain)
	if err != nil {
		return nil, 0, err
	}
	if !bytes.Equal(again, ct) {
		return nil, 0, errors.New("encrypting the restored plaintext does not give the ciphertext back")
	}
	return plain, returned, nil
}

// encryptBlock returns the raw Rijndael-256 encryption of one 32-byte block.
//
// The library exports no single-block call and fixes the IV inside New. The
// second block of a two-block CBC message does the work: with c1 the first
// ciphertext block, the plaintext block x xor c1 encrypts to E(x) whatever
// the IV is.
func (g *gen) encryptBlock(x []byte) ([]byte, error) {
	c1, err := g.encrypt(make([]byte, blockSize))
	if err != nil {
		return nil, err
	}
	msg := append(make([]byte, blockSize), xor(x, c1)...)
	both, err := g.encrypt(msg)
	if err != nil {
		return nil, err
	}
	if !bytes.Equal(both[:blockSize], c1) {
		return nil, errors.New("the first ciphertext block changed between two calls: the cipher keeps state")
	}
	return both[blockSize:], nil
}

// decryptBlock returns the raw Rijndael-256 decryption of one 32-byte block:
// after a zero first block, CBC decrypts x to D(x) xor zeros.
func (g *gen) decryptBlock(x []byte) ([]byte, error) {
	plain, _, err := g.decrypt(append(make([]byte, blockSize), x...))
	if err != nil {
		return nil, err
	}
	return plain[blockSize:], nil
}

func xor(a, b []byte) []byte {
	out := make([]byte, len(a))
	for i := range a {
		out[i] = a[i] ^ b[i]
	}
	return out
}
