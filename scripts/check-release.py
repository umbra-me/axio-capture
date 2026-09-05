"""Fail before packaging when a tag and product versions disagree."""
import json
import pathlib
import sys
import tomllib
root = pathlib.Path(__file__).resolve().parents[1]
versions = [json.loads((root / name).read_text())["version"] for name in ("package.json", "src-tauri/tauri.conf.json")]
versions.append(tomllib.loads((root / "src-tauri/Cargo.toml").read_text())["package"]["version"])
if len(set(versions)) != 1 or (len(sys.argv) > 1 and sys.argv[1] != "v" + versions[0]):
    sys.exit(f"Release tag/version mismatch: {sys.argv[1:]} / {versions}")
print("Release versions agree:", versions[0])
