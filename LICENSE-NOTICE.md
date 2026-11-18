# Licence provenance

- Mod: **Eternal Starlight** `0.9.1+1.21.1+neoforge`, by **LeoMinecraftModding**
  (`LeoLezury` in the jar's `META-INF/neoforge.mods.toml`) —
  <https://github.com/LeoMinecraftModding/eternal-starlight>.
- Licence: **MIT**, read from the jar itself:
  `META-INF/neoforge.mods.toml` line 4, `license = "MIT"` (rank 0 in the
  campaign's licence order). The upstream repository's `LICENSE` is the MIT text
  reproduced in this port's `LICENSE`, `Copyright (c) 2024
  LeoMinecraftModding`.
- The jar ships no `LICENSE` file of its own; the repository's text above is the
  licence of both the Java original and this clean-room Rust port.
- Content: the block ids, blockstates, models, textures and loot tables under
  `assets/eternal_starlight/` and `data/eternal_starlight/` are the original's
  own files, carried by `tools/carry_content.py` and converted to Minecraft
  26.3's formats where the campaign's pack-upgrade rules require it (see that
  tool's `ADAPTATIONS` and `notes.md`).
- This port is **local-only until the maintainer approves publishing**
  (`ARCHITECTURE.md` §8). MIT makes publication possible; it does not grant it.
