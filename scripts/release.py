# Usage:
#   python release.py patch              # 0.1.0 -> 0.1.1
#   python release.py minor              # 0.1.0 -> 0.2.0
#   python release.py major              # 0.1.0 -> 1.0.0
#   python release.py 1.2.3              # explicit version
#   python release.py patch --dry-run    # show what would happen, do nothing

import argparse
import re
import subprocess
import sys
from pathlib import Path

CARGO_TOML = Path("Cargo.toml")


def run(cmd: list[str], dry_run: bool = False, check: bool = True) -> str:
    print(f"+ {' '.join(cmd)}")
    if dry_run:
        return ""
    result = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if check and result.returncode != 0:
        print(result.stdout)
        print(result.stderr, file=sys.stderr)
        sys.exit(f"Command failed: {' '.join(cmd)}")
    return result.stdout.strip()


def get_current_version() -> str:
    if not CARGO_TOML.exists():
        sys.exit("Cargo.toml not found in current dir")
    for line in CARGO_TOML.read_text().splitlines():
        m = re.match(r'^\s*version\s*=\s*"([^"]+)"', line)
        if m:
            return m.group(1)
    sys.exit('Could not find a `version = "X.Y.Z"` line in Cargo.toml')


def bump_version(current: str, part: str) -> str:
    m = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", current)
    if not m:
        sys.exit(f"Current version '{current}' isn't plain semver (X.Y.Z)")
    major, minor, patch = map(int, m.groups())
    if part == "major":
        return f"{major + 1}.0.0"
    if part == "minor":
        return f"{major}.{minor + 1}.0"
    if part == "patch":
        return f"{major}.{minor}.{patch + 1}"
    sys.exit(f"Unknown bump type: {part}")


def resolve_new_version(current: str, target: str) -> str:
    if target in ("major", "minor", "patch"):
        return bump_version(current, target)
    if re.fullmatch(r"\d+\.\d+\.\d+", target):
        return target
    sys.exit(f"Invalid version argument: '{target}' (use 'major', 'minor', 'patch', or X.Y.Z)")


def write_new_version(new_version: str, dry_run: bool) -> None:
    text = CARGO_TOML.read_text()
    new_text, n = re.subn(
        r'(?m)^(\s*version\s*=\s*)"[^"]+"',
        rf'\1"{new_version}"',
        text,
        count=1,
    )
    if n == 0:
        sys.exit("Failed to find version line to replace in Cargo.toml")
    print(f"Updating Cargo.toml: version -> {new_version}")
    if not dry_run:
        CARGO_TOML.write_text(new_text)


def check_git_clean() -> None:
    status = run(["git", "status", "--porcelain"], dry_run=False)
    if status:
        sys.exit("Working tree is not clean. Commit or stash changes first:\n" + status)


def confirm(prompt: str, dry_run: bool) -> None:
    if dry_run:
        return
    answer = input(f"{prompt} [y/N] ").strip().lower()
    if answer != "y":
        sys.exit("Aborted.")


def main() -> None:
    parser = argparse.ArgumentParser(description="Bump version, tag, publish and push to trigger the release build.")
    parser.add_argument("bump", help="major | minor | patch | explicit version (e.g 1.2.3)")
    parser.add_argument("--dry-run", action="store_true", help="show steps without doing them")
    args = parser.parse_args()

    dry_run = args.dry_run
    if dry_run:
        print("--- DRY RUN: no changes will be made ---\n")

    current_version = get_current_version()
    new_version = resolve_new_version(current_version, args.bump)
    tag_name = f"v{new_version}"

    print(f"Current version: {current_version}")
    print(f"New version:     {new_version}")
    print(f"Tag:             {tag_name}\n")

    if not dry_run:
        check_git_clean()

    confirm(f"Proceed with releasing {tag_name}?", dry_run)

    write_new_version(new_version, dry_run)
    run(["git", "add", "Cargo.toml"], dry_run=dry_run)
    run(["cargo", "build", "--all-features"], dry_run=dry_run, check=False)
    run(["git", "add", "Cargo.lock"], dry_run=dry_run)
    run(["git", "commit", "-m", f"release {new_version}"], dry_run=dry_run)

    run(["git", "tag", f"{tag_name}"], dry_run=dry_run)
    run(["git", "push"], dry_run=dry_run)
    run(["git", "push", "--tags"], dry_run=dry_run)

    run(["cargo", "publish", "--all-features"], dry_run=dry_run)

    print(f"\nPushed {tag_name} — GitHub Actions will now build binaries and create the release.")
    if dry_run:
        print("(This was a dry run — nothing was actually changed.)")


if __name__ == "__main__":
    main()
