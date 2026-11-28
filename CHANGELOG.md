# Changelog

## 0.1.0 (2026-11-28)


### ⚠ BREAKING CHANGES

* **worldgen:** Worlds made under Generator versions 1 and 2 no longer open.
* **worldgen:** terrain and water, generator version 3 ([#63](https://github.com/mkappworks-dev/tesserae/issues/63))

### Features

* **audio:** add server-driven sound, biome soundscapes and mixing ([#58](https://github.com/mkappworks-dev/tesserae/issues/58)) ([26db997](https://github.com/mkappworks-dev/tesserae/commit/26db99780fdc1700dac80339754957bf2f7dc19a))
* **camera:** add the player figure and over-the-shoulder third-person view ([#29](https://github.com/mkappworks-dev/tesserae/issues/29)) ([0c913c3](https://github.com/mkappworks-dev/tesserae/commit/0c913c3c529d6123a074910c474b2966385599b7))
* chunk storage, greedy mesher, and project scaffolding ([88b6cb6](https://github.com/mkappworks-dev/tesserae/commit/88b6cb681315de855b0aab8ed91ba1cff9e06aeb))
* **client:** game-style menus, World and Mod previews, and the app icon ([#73](https://github.com/mkappworks-dev/tesserae/issues/73)) ([2f45144](https://github.com/mkappworks-dev/tesserae/commit/2f45144fe106d056d0c003811d7488ec9826acff))
* **client:** menus, Inventory and the Survival loop, matched to the prototype ([#57](https://github.com/mkappworks-dev/tesserae/issues/57)) ([2502752](https://github.com/mkappworks-dev/tesserae/commit/25027521aa62c669ace311084c59e9c43c973356))
* **controls:** add input ownership, remapping and native settings ([#24](https://github.com/mkappworks-dev/tesserae/issues/24)) ([4b86e43](https://github.com/mkappworks-dev/tesserae/commit/4b86e435f727efeef83cdd75c00aaf2f43276dd2))
* **controls:** controller input, navigation and settings ([#28](https://github.com/mkappworks-dev/tesserae/issues/28)) ([a2e2ac6](https://github.com/mkappworks-dev/tesserae/commit/a2e2ac6d5886ccc934c57bd44693584ef38f88c7))
* **controls:** desktop touch contacts and overlay ([#38](https://github.com/mkappworks-dev/tesserae/issues/38)) ([a20e8ad](https://github.com/mkappworks-dev/tesserae/commit/a20e8adcf7d5e8482f04f35ed375299aa969f4ed))
* **controls:** optional repeat place and remove while held ([#31](https://github.com/mkappworks-dev/tesserae/issues/31)) ([5d2f1e1](https://github.com/mkappworks-dev/tesserae/commit/5d2f1e128157819cccbaa89161b6f1f6d2d0c369))
* **core:** material table for emission, dimming and render class ([#9](https://github.com/mkappworks-dev/tesserae/issues/9)) ([7a9d313](https://github.com/mkappworks-dev/tesserae/commit/7a9d31322c52926f22fc6a95980381236f88ff79))
* **fluids:** fluid, lava and fire simulation with a 0.2 Mod contract ([#19](https://github.com/mkappworks-dev/tesserae/issues/19)) ([9992e8e](https://github.com/mkappworks-dev/tesserae/commit/9992e8e3aacc7795f1af4a242e5d310e993f219d))
* **fluids:** water is Game content, with a material menu and data-driven hotbar ([#20](https://github.com/mkappworks-dev/tesserae/issues/20)) ([212d900](https://github.com/mkappworks-dev/tesserae/commit/212d9008f33730ed2250ff620bb2fafced80284b))
* **games:** add demo Games and shared data Mods ([#52](https://github.com/mkappworks-dev/tesserae/issues/52)) ([ca45503](https://github.com/mkappworks-dev/tesserae/commit/ca455031ace668aef9e0cda2e5c66c28d0860b3b))
* incremental light propagation baked into the mesh ([#3](https://github.com/mkappworks-dev/tesserae/issues/3)) ([c573387](https://github.com/mkappworks-dev/tesserae/commit/c5733871b21c65d60bde09fcc2709dc3c018ffc2))
* **layout:** CAD-style layout placement with live preview ([#36](https://github.com/mkappworks-dev/tesserae/issues/36)) ([ef6835d](https://github.com/mkappworks-dev/tesserae/commit/ef6835d6ed92d029452171daada2e0bb3a8e7f9e))
* leave only air built in and preview declared content ([#21](https://github.com/mkappworks-dev/tesserae/issues/21)) ([5453619](https://github.com/mkappworks-dev/tesserae/commit/54536199cdd586c84dcf139a360cd02580e32368))
* **lod:** light coarse levels with a covered-column rule ([#15](https://github.com/mkappworks-dev/tesserae/issues/15)) ([9ec5571](https://github.com/mkappworks-dev/tesserae/commit/9ec5571fb9d50adda3de867e72e678aa6d30e8c0))
* **lod:** mip chain out to 4 km ([#13](https://github.com/mkappworks-dev/tesserae/issues/13)) ([14a8fae](https://github.com/mkappworks-dev/tesserae/commit/14a8fae5822354c46dc0c757e6295e090e240103))
* **lod:** step 22 LOD features — Impostors, Canopy cover and the Distance presets ([#69](https://github.com/mkappworks-dev/tesserae/issues/69)) ([811856a](https://github.com/mkappworks-dev/tesserae/commit/811856a648dd7061c0cc7553ddc22eb7d7856aff))
* **map:** add the Map, Compass and Waypoints ([#61](https://github.com/mkappworks-dev/tesserae/issues/61)) ([f792b24](https://github.com/mkappworks-dev/tesserae/commit/f792b244e8e634af8471a84c24cac198936212c0))
* **mods:** add the modding SDK and rename the demo Game to terra ([#51](https://github.com/mkappworks-dev/tesserae/issues/51)) ([e1a8da2](https://github.com/mkappworks-dev/tesserae/commit/e1a8da2179bae033627ce14bab99a1f45c89513c))
* **mods:** host sandboxed WASI Mods and the demo Game ([#18](https://github.com/mkappworks-dev/tesserae/issues/18)) ([ee77e54](https://github.com/mkappworks-dev/tesserae/commit/ee77e54593d6cc3fa0f26e34630f4e87015a6786))
* **mods:** install Games and Mods side by side; a World freezes its set ([#41](https://github.com/mkappworks-dev/tesserae/issues/41)) ([4247012](https://github.com/mkappworks-dev/tesserae/commit/424701268f3593777acb420e2c7fe5f39dab4f2f))
* **mods:** shared building Mods, and a Recipe needs an Obtainable Station ([#76](https://github.com/mkappworks-dev/tesserae/issues/76)) ([b6817a1](https://github.com/mkappworks-dev/tesserae/commit/b6817a1b4183f9085811136b2f5b646d24a59fc7))
* **movement:** add server-authoritative flight and physics movement ([#26](https://github.com/mkappworks-dev/tesserae/issues/26)) ([96ea33e](https://github.com/mkappworks-dev/tesserae/commit/96ea33eaf6fdea8d2c23f8c751852736c0394107))
* **movement:** buoyancy, drag and swimming in fluids ([#37](https://github.com/mkappworks-dev/tesserae/issues/37)) ([2e7ba92](https://github.com/mkappworks-dev/tesserae/commit/2e7ba9206f9d36ff4b4e3416ff26397911557d12))
* **plants:** vine crops, spreader floors, fields and the worlds card menu ([#60](https://github.com/mkappworks-dev/tesserae/issues/60)) ([0074d65](https://github.com/mkappworks-dev/tesserae/commit/0074d65a006a08955e0e73dd2c52992df9be4007))
* **render:** blended pass at every LOD level, lakes, cutout leaves and non-solid water ([#16](https://github.com/mkappworks-dev/tesserae/issues/16)) ([a95567d](https://github.com/mkappworks-dev/tesserae/commit/a95567d2f5b5e7f790c33aa396f4d20a4565b6da))
* **render:** distance fog on every LOD camera ([#17](https://github.com/mkappworks-dev/tesserae/issues/17)) ([e52c509](https://github.com/mkappworks-dev/tesserae/commit/e52c5097c0438192dec4d1401f5196b17fdbd2b1))
* **render:** spawn meshed chunks with a MaterialExtension and fly camera ([#1](https://github.com/mkappworks-dev/tesserae/issues/1)) ([fee08eb](https://github.com/mkappworks-dev/tesserae/commit/fee08ebb35eac986e052b2cf246a29487e4f32f8))
* **render:** underwater view for fluid immersion ([#34](https://github.com/mkappworks-dev/tesserae/issues/34)) ([f5f2a86](https://github.com/mkappworks-dev/tesserae/commit/f5f2a861c45356b279280ed3bb043c1e05287255))
* **save:** keep three commits, write each record once in binary packs ([#64](https://github.com/mkappworks-dev/tesserae/issues/64)) ([ec02b66](https://github.com/mkappworks-dev/tesserae/commit/ec02b66f627da7ca3acb2bd28e35554bd5ee7bb5))
* **save:** persist Worlds across eviction and restart ([#23](https://github.com/mkappworks-dev/tesserae/issues/23)) ([aa3c2b1](https://github.com/mkappworks-dev/tesserae/commit/aa3c2b15b10f50c46fbe65ee93027e23054e2d7b))
* **server:** add dropped items, pickup and Carried ([#55](https://github.com/mkappworks-dev/tesserae/issues/55)) ([7fb8fe0](https://github.com/mkappworks-dev/tesserae/commit/7fb8fe0d1c35765fc70bd66e578227f3630ba94a))
* **server:** check an Edit's ray origin against the Server's Player ([#7](https://github.com/mkappworks-dev/tesserae/issues/7)) ([9cc90e5](https://github.com/mkappworks-dev/tesserae/commit/9cc90e53f98b38839d7bc7b7e673b7a77b88b2e9))
* **shape:** connected shapes and collision taller than the cell ([#12](https://github.com/mkappworks-dev/tesserae/issues/12)) ([c41c8c2](https://github.com/mkappworks-dev/tesserae/commit/c41c8c21f22b11c6e5930a0e2a5a4453365053f4))
* shaped voxels with a second mesher pass, per-shape collision and opacity ([#4](https://github.com/mkappworks-dev/tesserae/issues/4)) ([047725f](https://github.com/mkappworks-dev/tesserae/commit/047725f3a683cc730638949c0bbe3e418994371e))
* **sky:** day and night cycle ([#77](https://github.com/mkappworks-dev/tesserae/issues/77)) ([454d4f9](https://github.com/mkappworks-dev/tesserae/commit/454d4f9690d600f7dc8aaaac9217beae8141ca16))
* **structures:** Templates placed by worldgen and the place Tool ([#62](https://github.com/mkappworks-dev/tesserae/issues/62)) ([d3bf6bc](https://github.com/mkappworks-dev/tesserae/commit/d3bf6bc4af84b5f500f618875b539c485ac607c4))
* **tools:** explicit Use and a first measuring tool ([#32](https://github.com/mkappworks-dev/tesserae/issues/32)) ([0d8c303](https://github.com/mkappworks-dev/tesserae/commit/0d8c303b60c9eacf3d76cc532e6976e0e80eb468))
* **use:** crafting, Stations, Containers and Doors ([#59](https://github.com/mkappworks-dev/tesserae/issues/59)) ([de2535a](https://github.com/mkappworks-dev/tesserae/commit/de2535a28a44d5dc468a4ee20d6aab0732693308))
* **water:** water that looks natural ([#78](https://github.com/mkappworks-dev/tesserae/issues/78)) ([fd81dde](https://github.com/mkappworks-dev/tesserae/commit/fd81dded93a74180af37730d16935431929c7b72))
* worldgen and chunk streaming behind the client/server split ([#2](https://github.com/mkappworks-dev/tesserae/issues/2)) ([2a616fe](https://github.com/mkappworks-dev/tesserae/commit/2a616fee03045a23047c11396dbb4918128d5b1b))
* **worldgen:** add biomes, climate and a pinned generator version ([#56](https://github.com/mkappworks-dev/tesserae/issues/56)) ([4e905eb](https://github.com/mkappworks-dev/tesserae/commit/4e905eb062593912873ce9ec6d4570e93b8b4c7b))
* **worldgen:** caves behind a bounded early-out ([#6](https://github.com/mkappworks-dev/tesserae/issues/6)) ([f232c4e](https://github.com/mkappworks-dev/tesserae/commit/f232c4ef60bb055618534602eec8b042d5a130e3))
* **worldgen:** slope-smoothing stairs and slabs where the ground steps one Voxel ([#11](https://github.com/mkappworks-dev/tesserae/issues/11)) ([73922ce](https://github.com/mkappworks-dev/tesserae/commit/73922cefcadfaa324a51c2a4e6243223a071f167))
* **worldgen:** terrain and water, generator version 3 ([#63](https://github.com/mkappworks-dev/tesserae/issues/63)) ([b88e7c1](https://github.com/mkappworks-dev/tesserae/commit/b88e7c12be132b06815d17987eda012ee23be3e4))
* **worldgen:** version 3 only, and deterministic on every platform ([#65](https://github.com/mkappworks-dev/tesserae/issues/65)) ([16119ee](https://github.com/mkappworks-dev/tesserae/commit/16119eea48fc86c39b912a3d37d7d6acd6cbe743))
* **world:** open and leave Worlds in-process as sessions ([#39](https://github.com/mkappworks-dev/tesserae/issues/39)) ([039e3c5](https://github.com/mkappworks-dev/tesserae/commit/039e3c511f93fa7206b3b5539b9db0a019792b36))
* **world:** World list window and controller follow-ups ([#33](https://github.com/mkappworks-dev/tesserae/issues/33)) ([8539496](https://github.com/mkappworks-dev/tesserae/commit/8539496ec18353d8594acc9f8a24d70d4356fe86))
* **world:** World list, creation and seed selection ([#30](https://github.com/mkappworks-dev/tesserae/issues/30)) ([446e984](https://github.com/mkappworks-dev/tesserae/commit/446e9845e41d0d94e2ac835ddbde999c229f6ff3))


### Bug Fixes

* **controls:** retry failed hotbar profile writes ([#25](https://github.com/mkappworks-dev/tesserae/issues/25)) ([f085331](https://github.com/mkappworks-dev/tesserae/commit/f085331da9815ab31daf32365b5826acf77b1e8e))
* **light:** a face two partial shapes jointly cover blocks light ([#8](https://github.com/mkappworks-dev/tesserae/issues/8)) ([b5fc088](https://github.com/mkappworks-dev/tesserae/commit/b5fc088b0de8f9a5cb30234c4a4c551f4d4bf69d))
* **player:** lift a physics respawn out of Voxels at the spawn ([#35](https://github.com/mkappworks-dev/tesserae/issues/35)) ([918997b](https://github.com/mkappworks-dev/tesserae/commit/918997b53eecfd53b4c1f57add93532e7d73efa4))
* **render:** colour, art and light ([#75](https://github.com/mkappworks-dev/tesserae/issues/75)) ([6a81409](https://github.com/mkappworks-dev/tesserae/commit/6a81409f0caaf5d6f63a8cbb531d6b3faf5c6ab5))
* **render:** mipmapped Material textures stop distant shimmer ([#74](https://github.com/mkappworks-dev/tesserae/issues/74)) ([0b03c59](https://github.com/mkappworks-dev/tesserae/commit/0b03c59475018553eda50930cfc3c342695decfd))


### Performance

* **render:** cheaper sun shadows and shared Chunk placeholders ([#68](https://github.com/mkappworks-dev/tesserae/issues/68)) ([65037cd](https://github.com/mkappworks-dev/tesserae/commit/65037cdd1cc7b962bee8e6eecb0e80d1d3bc168b))
* **render:** draw Chunk meshes from one record per quad ([#66](https://github.com/mkappworks-dev/tesserae/issues/66)) ([60ee1ca](https://github.com/mkappworks-dev/tesserae/commit/60ee1ca701ab6287c5ae94413a0bd531d4c7359d))
* **render:** reduce GPU pixel cost and add presentation settings ([#50](https://github.com/mkappworks-dev/tesserae/issues/50)) ([df22364](https://github.com/mkappworks-dev/tesserae/commit/df22364087ab68ce649209620feae80e149e134a))
* **server:** land Mod builds as one write batch with one relight ([#54](https://github.com/mkappworks-dev/tesserae/issues/54)) ([2e86502](https://github.com/mkappworks-dev/tesserae/commit/2e865027bb0a9167ce8a47f38e871963636b248e))
* **streaming:** halve frame time and draw no gaps while streaming ([#47](https://github.com/mkappworks-dev/tesserae/issues/47)) ([bb29ad3](https://github.com/mkappworks-dev/tesserae/commit/bb29ad38a919b592d0a1b2c2189712a8c296084b))
* **worldgen:** cheaper version 3 generation and a sooner terrain fill ([#67](https://github.com/mkappworks-dev/tesserae/issues/67)) ([5285969](https://github.com/mkappworks-dev/tesserae/commit/5285969a1faad4b7f8ebe25bafc2743ab9aecbd3))


### Technical

* **build:** line-tables-only debug info in the dev profile ([#10](https://github.com/mkappworks-dev/tesserae/issues/10)) ([e7820e3](https://github.com/mkappworks-dev/tesserae/commit/e7820e343ff4bea158b4cb0d518a497d07426007))
* **server:** run chunk streaming off the main thread ([#5](https://github.com/mkappworks-dev/tesserae/issues/5)) ([d8b3f31](https://github.com/mkappworks-dev/tesserae/commit/d8b3f31ae633a8d3b19b857e9cf3125e68c28a03))


### Documentation

* add MVP roadmap to the design build order ([#27](https://github.com/mkappworks-dev/tesserae/issues/27)) ([6791f48](https://github.com/mkappworks-dev/tesserae/commit/6791f48bd868904509b92a8f7148caa86954759a))
* add the CLA legal-review spec and tickets ([776a293](https://github.com/mkappworks-dev/tesserae/commit/776a293b26404264c0a6a0b8386fecdbe1b70cce))
* **design:** order the road to 1.0 by dependency and add a status column ([#46](https://github.com/mkappworks-dev/tesserae/issues/46)) ([d46b322](https://github.com/mkappworks-dev/tesserae/commit/d46b3220a77000cbeb5479ca4f122b01ede7be83))
* **lod:** ticket for Edits and emitters in coarse levels ([#14](https://github.com/mkappworks-dev/tesserae/issues/14)) ([f57f72d](https://github.com/mkappworks-dev/tesserae/commit/f57f72d65175b4b5532e94e90993ddc7463b9261))
* refresh the execution plan ([#45](https://github.com/mkappworks-dev/tesserae/issues/45)) ([ba97761](https://github.com/mkappworks-dev/tesserae/commit/ba9776143d8fb4a647a4b9537ff46fd297a97989))
* **scratch:** close finished specs and say why connectivity stays profiling-only ([#48](https://github.com/mkappworks-dev/tesserae/issues/48)) ([ff0e3b7](https://github.com/mkappworks-dev/tesserae/commit/ff0e3b753aa4f4e328456dcdb658e6a8cd33f84e))
* **scratch:** measure and triage World session memory growth ([#40](https://github.com/mkappworks-dev/tesserae/issues/40)) ([6bf5c87](https://github.com/mkappworks-dev/tesserae/commit/6bf5c872b231971e536abb997553958caf3eda64))
* **workflow:** rename glossary, pin skills plugin, merge PR body shape ([#42](https://github.com/mkappworks-dev/tesserae/issues/42)) ([cc64b8b](https://github.com/mkappworks-dev/tesserae/commit/cc64b8b40bd54af4d47d5b47de0e50d6434c9da7))


### Build

* **deps:** bump bevy to 0.20.1 and update dependencies ([#49](https://github.com/mkappworks-dev/tesserae/issues/49)) ([0f21600](https://github.com/mkappworks-dev/tesserae/commit/0f216005316abff1907913ea921c74baf9127795))


### CI

* **release:** ship the Games and Mods in every binary archive ([#70](https://github.com/mkappworks-dev/tesserae/issues/70)) ([60fcbc6](https://github.com/mkappworks-dev/tesserae/commit/60fcbc680970dc162a352137e31a84daec9176f5))
* **release:** start at 0.1.0 and sync Cargo.lock on the release PR ([#72](https://github.com/mkappworks-dev/tesserae/issues/72)) ([38a92cd](https://github.com/mkappworks-dev/tesserae/commit/38a92cd7504b9cd6c523b0cf7de633e67184fd77))
* share the guest target dir and move bench build to its own job ([#43](https://github.com/mkappworks-dev/tesserae/issues/43)) ([8e624fd](https://github.com/mkappworks-dev/tesserae/commit/8e624fd9d3d293467fed95ac1fbd682290bae64c))
* **test:** run tests under nextest and optimise tesserae-core ([#44](https://github.com/mkappworks-dev/tesserae/issues/44)) ([ed4e6a2](https://github.com/mkappworks-dev/tesserae/commit/ed4e6a28ead9ca89b0d0d5f5946c705922455186))
