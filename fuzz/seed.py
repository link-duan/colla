"""Seed both strict decoder targets from the shared version-2 golden fixtures."""

import json
from pathlib import Path


root = Path(__file__).resolve().parent
fixtures = json.loads((root.parent / "golden/v2.json").read_text())
count = 0
for fixture in fixtures:
    for field, value in fixture.items():
        if not isinstance(value, str) or not value.startswith("434f4c4c410200"):
            continue
        encoded = bytes.fromhex(value)
        target = "decode_value" if encoded[7] in (1, 3) else "decode_change"
        directory = root / "corpus" / target
        directory.mkdir(parents=True, exist_ok=True)
        (directory / f"golden-{fixture['name']}-{field}").write_bytes(encoded)
        count += 1
print(f"Seeded {count} canonical version-2 inputs")
