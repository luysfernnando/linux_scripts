package main

import (
	"bufio"
	"os"
	"runtime"
	"strings"
)

// Family identifies which package-manager toolchain a system uses.
type Family string

const (
	FamilyArch    Family = "arch"
	FamilyDebian  Family = "debian"
	FamilyFedora  Family = "fedora"
	FamilySuse    Family = "suse"
	FamilyDarwin  Family = "darwin"
	FamilyWindows Family = "windows"
	FamilyUnknown Family = "unknown"
)

// DetectFamily figures out the OS/distro family driving package-manager choice.
func DetectFamily() Family {
	switch runtime.GOOS {
	case "darwin":
		return FamilyDarwin
	case "windows":
		return FamilyWindows
	case "linux":
		return detectLinuxFamily("/etc/os-release")
	default:
		return FamilyUnknown
	}
}

func detectLinuxFamily(osReleasePath string) Family {
	f, err := os.Open(osReleasePath)
	if err != nil {
		return FamilyUnknown
	}
	defer f.Close()

	var id, idLike string
	scanner := bufio.NewScanner(f)
	for scanner.Scan() {
		line := scanner.Text()
		switch {
		case strings.HasPrefix(line, "ID="):
			id = unquote(strings.TrimPrefix(line, "ID="))
		case strings.HasPrefix(line, "ID_LIKE="):
			idLike = unquote(strings.TrimPrefix(line, "ID_LIKE="))
		}
	}

	haystack := id + " " + idLike
	switch {
	case containsAny(haystack, "arch", "cachyos", "manjaro", "endeavouros"):
		return FamilyArch
	case containsAny(haystack, "debian", "ubuntu", "mint", "pop"):
		return FamilyDebian
	case containsAny(haystack, "fedora", "rhel", "centos"):
		return FamilyFedora
	case containsAny(haystack, "suse"):
		return FamilySuse
	default:
		return FamilyUnknown
	}
}

func unquote(s string) string {
	return strings.Trim(strings.TrimSpace(s), `"`)
}

func containsAny(haystack string, needles ...string) bool {
	for _, n := range needles {
		if strings.Contains(haystack, n) {
			return true
		}
	}
	return false
}
