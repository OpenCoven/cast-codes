'''
Generates the `ExternalFontFamily` definitions used in `app/src/font_fallback.rs`.
These definitions contain the URLs to each external fallback font we use in Warp.
Generated code is sent to stdout.

This script reads fallback font files from a local directory and generates the
code required to initialize static references for each font family.

Usage:
1. Put fallback fonts under `downloaded_fonts/<family>/<font>.ttf`.
2. Run `python3 generate_families.py`.
3. Manually inspect the name for each font. The script will generate the name in
   title-case, but this isn't correct for some fonts (e.g. Noto Sans SC).
'''

from collections import defaultdict
from pathlib import Path

FONT_DIR = Path("downloaded_fonts")


def list_fonts():
    if not FONT_DIR.exists():
        raise SystemExit(f"Missing local fallback font directory: {FONT_DIR}")

    font_paths = sorted(FONT_DIR.rglob("*.ttf"))
    if not font_paths:
        raise SystemExit(f"No .ttf files found under {FONT_DIR}")
    return font_paths


def generate_families(font_paths):
    family_map = defaultdict(list)
    for path in font_paths:
        rel_path = path.relative_to(FONT_DIR)
        if len(rel_path.parts) < 2:
            raise SystemExit(f"Expected <family>/<font>.ttf under {FONT_DIR}, got {rel_path}")
        family_name = rel_path.parts[0]
        family_map[family_name].append(path.name)

    for family_name, font_names in family_map.items():
        print_family(family_name, font_names)


def indent_level(level, s):
    indent = "    " * level
    return indent + s


def print_family(family_name, font_names):
    variable_name = family_name.replace('-', '_').upper()
    title_case_name = family_name.replace('-', ' ').title()

    print(f"static ref {variable_name}: ExternalFontFamily = ExternalFontFamily {{")
    # Title-case is not correct for some fonts, e.g. "Noto Sans SC", so we add
    # a todo to make any manual adjustments.
    print(indent_level(1, f"name: \"{title_case_name}\", // TODO: double-check the title is correct"))
    print(indent_level(1, "font_urls: Arc::new(vec!["))
    for font_name in font_names:
        print(indent_level(2, f"url_for_font(\"{family_name}\", \"{font_name}\"),"))
    print(indent_level(1, "]),"))
    print("};")


def main():
    font_uris = list_fonts()
    generate_families(font_uris)


if __name__ == "__main__":
    main()
