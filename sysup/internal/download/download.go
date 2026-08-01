// Package download is the shared "fetch a URL, optionally verify a
// checksums.txt entry, extract a tar.gz" logic used by both the polkit
// setup flow (downloading sysup-worker from a GitHub release) and the
// GitKraken installer — previously two independent hand-rolled
// implementations of the same GET+extract steps.
package download

import (
	"archive/tar"
	"compress/gzip"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// Get fetches url and returns the full response body.
func Get(url string) ([]byte, error) {
	client := &http.Client{Timeout: 60 * time.Second}
	resp, err := client.Get(url)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("GET %s: status %s", url, resp.Status)
	}
	return io.ReadAll(resp.Body)
}

// GetToFile fetches url and writes the response body straight to dest.
func GetToFile(url, dest string) error {
	resp, err := http.Get(url)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("GET %s: status %s", url, resp.Status)
	}
	f, err := os.Create(dest)
	if err != nil {
		return err
	}
	defer f.Close()
	_, err = io.Copy(f, resp.Body)
	return err
}

// VerifyChecksum checks data's sha256 against the entry for assetName in a
// `sha256  filename` formatted checksums.txt (the format GoReleaser emits).
func VerifyChecksum(checksumsTxt, assetName string, data []byte) error {
	var want string
	for _, line := range strings.Split(checksumsTxt, "\n") {
		fields := strings.Fields(line)
		if len(fields) == 2 && fields[1] == assetName {
			want = fields[0]
			break
		}
	}
	if want == "" {
		return fmt.Errorf("checksum de %s não encontrado em checksums.txt", assetName)
	}
	sum := sha256.Sum256(data)
	got := hex.EncodeToString(sum[:])
	if got != want {
		return fmt.Errorf("checksum de %s não bate (esperado %s, obtido %s)", assetName, want, got)
	}
	return nil
}

// ExtractSingleFile pulls just wantName out of a tar.gz held in memory and
// writes it to a fresh temp file (0755), returning that path — used when
// only one binary inside the archive is actually needed (sysup-worker).
func ExtractSingleFile(tgz []byte, wantName string) (string, error) {
	gz, err := gzip.NewReader(strings.NewReader(string(tgz)))
	if err != nil {
		return "", err
	}
	defer gz.Close()

	tr := tar.NewReader(gz)
	for {
		hdr, err := tr.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			return "", err
		}
		if filepath.Base(hdr.Name) != wantName {
			continue
		}
		out := filepath.Join(os.TempDir(), fmt.Sprintf("%s-%d", wantName, os.Getpid()))
		f, err := os.OpenFile(out, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, 0o755)
		if err != nil {
			return "", err
		}
		if _, err := io.Copy(f, tr); err != nil {
			f.Close()
			return "", err
		}
		f.Close()
		return out, nil
	}
	return "", fmt.Errorf("%s não encontrado no arquivo baixado", wantName)
}

// ExtractTarGz extracts every file/dir/symlink in the archive at
// archivePath into destDir, preserving structure and file modes — used
// when the whole tree is needed (GitKraken's install layout), unlike
// ExtractSingleFile.
func ExtractTarGz(archivePath, destDir string) error {
	f, err := os.Open(archivePath)
	if err != nil {
		return err
	}
	defer f.Close()

	gz, err := gzip.NewReader(f)
	if err != nil {
		return err
	}
	defer gz.Close()

	tr := tar.NewReader(gz)
	for {
		hdr, err := tr.Next()
		if err == io.EOF {
			return nil
		}
		if err != nil {
			return err
		}
		target := filepath.Join(destDir, hdr.Name)

		switch hdr.Typeflag {
		case tar.TypeDir:
			if err := os.MkdirAll(target, 0o755); err != nil {
				return err
			}
		case tar.TypeReg:
			if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
				return err
			}
			out, err := os.OpenFile(target, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, os.FileMode(hdr.Mode))
			if err != nil {
				return err
			}
			if _, err := io.Copy(out, tr); err != nil {
				out.Close()
				return err
			}
			out.Close()
		case tar.TypeSymlink:
			if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
				return err
			}
			_ = os.Remove(target)
			if err := os.Symlink(hdr.Linkname, target); err != nil {
				return err
			}
		}
	}
}
