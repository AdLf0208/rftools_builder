# Licence notice — RFTools Builder

- **Upstream:** RFTools Builder 1.21-7.0.8, <https://github.com/McJtyMods/RFToolsBuilder>.
- **Licence as read from the shipped jar** (`META-INF/neoforge.mods.toml`):
  `license="MIT License https://github.com/McJtyMods/RFToolsBuilder/blob/1.21_neo/LICENCE"`;
  `python3 ports/tools/audit_licences.py --mod rftoolsbuilder --fetch` reads the
  repository LICENCE as **MIT**, rank 0. The port `LICENSE` carries that MIT
  text, `Copyright (c) 2019 McJtyMods`.
- **What travels with the artifact:** `LICENSE` and this notice are packaged
  inside the distributed `.hmod` at the package root, so an artifact-only
  download carries them (MIT's notice condition).
- **Carried art:** the blockstate, the block and item models, the Builder's
  front texture (`textures/block/machinebuilder.png`) and the card's texture
  (`textures/item/shapecarditem.png`) are the upstream mod's own files, under
  the same MIT licence, with their sizes and SHA-256 recorded in `CONTENT.md`.
  The three machine casing textures (`textures/block/base/machine*.png`) come
  from **RFTools Base** 1.21-6.0.11 (McJtyMods/RFToolsBase, MIT, same author),
  as the sibling ports do, so the Builder renders without that mod.
- **Status:** local-only until the maintainer approves publishing
  (ARCHITECTURE.md §8). MIT permits redistribution with its notice, so the port
  is publishable under MIT with this source.
