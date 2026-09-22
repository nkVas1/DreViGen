"""Package numbered final assets; archive untouched native generations outside the repo."""
from pathlib import Path
import hashlib
import json
import sys
import zipfile

sys.stdout.reconfigure(encoding="utf-8")
root = Path(__file__).resolve().parents[2]
delivery = root / "assets/generated/delivery/drevigen-herbarium-vivum-2026-09-22.zip"
delivery.parent.mkdir(parents=True, exist_ok=True)
include = [
    "assets/source", "assets/demo", "assets/fonts", "assets/identity", "assets/generated/01",
    "assets/generated/02", "assets/generated/03", "assets/generated/04", "assets/generated/05",
    "assets/generated/06", "assets/generated/07", "assets/generated/08", "assets/generated/09",
    "assets/generated/10", "assets/generated/11", "assets/generated/12", "packages/ui/icons",
    "tools/assets", "tools/textures", "docs/03-design", "LICENSE",
    "assets/catalog.html", "assets/manifest.json", "assets/DELIVERY.md", "assets/README.md",
    "assets/verification.json", "assets/browser-verification.json", "assets/integration.css",
]
files = set()
for relative in include:
    source = root / relative
    if source.is_file():
        files.add(source)
    elif source.is_dir():
        files.update(p for p in source.rglob("*") if p.is_file() and "node_modules" not in p.parts and "__pycache__" not in p.parts)
with zipfile.ZipFile(delivery, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
    for source in sorted(files):
        archive.write(source, source.relative_to(root).as_posix())
with zipfile.ZipFile(delivery) as archive:
    assert archive.testzip() is None

master_zip = root.parent / "DreViGen-design-masters-2026-09-22.zip"
receipts = sorted((root / ".design-scratch/asset-generation").glob("*.json"))
master_count = 0
if receipts:
    with zipfile.ZipFile(master_zip, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=4) as archive:
        for receipt_file in receipts:
            receipt = json.loads(receipt_file.read_text(encoding="utf-8"))
            source = Path(receipt["original"])
            if not source.exists():
                continue
            name = receipt_file.stem
            archive.write(source, f"{receipt['group']}/{name}.png")
            archive.write(receipt_file, f"{receipt['group']}/{name}.json")
            master_count += 1
    with zipfile.ZipFile(master_zip) as archive:
        assert archive.testzip() is None

checksum = hashlib.sha256(delivery.read_bytes()).hexdigest()
delivery.with_suffix(".sha256").write_text(f"{checksum}  {delivery.name}\n", encoding="ascii")
print(json.dumps({"delivery": str(delivery), "files": len(files), "bytes": delivery.stat().st_size,
                  "sha256": checksum, "nativeMasters": master_count,
                  "masterArchive": str(master_zip) if master_count else None}, indent=2))
