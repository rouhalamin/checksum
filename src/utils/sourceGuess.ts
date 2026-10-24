// Small local (fully offline) heuristic: guess an official source domain
// from the file name only. Exactly matches the lookup table in CheckSum Rust source.

export const KNOWN_SOURCES: Array<[string, string]> = [
  ['kali', 'kali.org'],
  ['kubuntu', 'kubuntu.org'],
  ['xubuntu', 'xubuntu.org'],
  ['lubuntu', 'lubuntu.me'],
  ['ubuntu', 'ubuntu.com'],
  ['debian', 'debian.org'],
  ['fedora', 'getfedora.org'],
  ['centos', 'centos.org'],
  ['rocky', 'rockylinux.org'],
  ['almalinux', 'almalinux.org'],
  ['mint', 'linuxmint.com'],
  ['manjaro', 'manjaro.org'],
  ['archlinux', 'archlinux.org'],
  ['opensuse', 'opensuse.org'],
  ['tails', 'tails.net'],
  ['whonix', 'whonix.org'],
  ['parrot', 'parrotsec.org'],
  ['freebsd', 'freebsd.org'],
  ['openbsd', 'openbsd.org'],
  ['proxmox', 'proxmox.com'],
  ['vmware', 'vmware.com'],
  ['virtualbox', 'virtualbox.org'],
  ['firefox', 'mozilla.org'],
  ['vscode', 'code.visualstudio.com'],
  ['python', 'python.org'],
  ['nodejs', 'nodejs.org'],
  ['node-v', 'nodejs.org'],
  ['git-', 'git-scm.com'],
  ['7z', '7-zip.org'],
  ['nmap', 'nmap.org'],
  ['wireshark', 'wireshark.org'],
  ['docker', 'docker.com'],
  ['blender', 'blender.org'],
  ['gimp', 'gimp.org'],
  ['libreoffice', 'libreoffice.org'],
  ['obs-studio', 'obsproject.com'],
  ['putty', 'putty.org'],
  ['filezilla', 'filezilla-project.org'],
  ['npp.', 'notepad-plus-plus.org'],
];

export function guessSource(fileName: string): { domain: string; url: string | null } {
  const lower = fileName.toLowerCase();
  for (const [keyword, domain] of KNOWN_SOURCES) {
    if (lower.includes(keyword)) {
      return {
        domain,
        url: `https://${domain}`,
      };
    }
  }
  return { domain: 'Unknown', url: null };
}

export function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B';
  const KB = 1024;
  const MB = KB * 1024;
  const GB = MB * 1024;

  if (bytes >= GB) {
    return `${(bytes / GB).toFixed(2)} GB`;
  } else if (bytes >= MB) {
    return `${(bytes / MB).toFixed(2)} MB`;
  } else if (bytes >= KB) {
    return `${(bytes / KB).toFixed(2)} KB`;
  } else {
    return `${bytes} B`;
  }
}

export function formatSpeed(bytesPerSec: number): string {
  const valid = Math.max(0, bytesPerSec || 0);
  return `${formatBytes(valid)}/s`;
}

export function formatElapsed(seconds: number): string {
  if (seconds >= 60) {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}m ${secs}s`;
  }
  return `${seconds.toFixed(1)}s`;
}
