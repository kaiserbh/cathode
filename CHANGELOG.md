# Changelog

## [0.5.2](https://github.com/kaiserbh/cathode/compare/v0.5.1...v0.5.2) (2026-06-25)


### Features

* linux backend ([#19](https://github.com/kaiserbh/cathode/issues/19)) ([f198fec](https://github.com/kaiserbh/cathode/commit/f198fecc130f07172b5de51bbee537db086c008a))

## [0.5.1](https://github.com/kaiserbh/cathode/compare/v0.5.0...v0.5.1) (2026-06-24)


### Bug Fixes

* bundle libmpv runtime DLLs in the Windows installer ([dcf87c9](https://github.com/kaiserbh/cathode/commit/dcf87c931be2e6ce0718b09a6d3e255742d5d9cf))

## [0.5.0](https://github.com/kaiserbh/cathode/compare/v0.1.0...v0.5.0) (2026-06-24)


### ⚠ BREAKING CHANGES

* **core:** the crate is now imported as `cathode_core::` and built with `cargo test -p cathode-core`.

### Features

* **brand:** add generated Android and iOS launcher icons ([f5ebd81](https://github.com/kaiserbh/cathode/commit/f5ebd81b284dd41008a9e45da1a9a4144009ec6d))
* **brand:** new Cathode app icon (glowing CRT with play) ([c59f42a](https://github.com/kaiserbh/cathode/commit/c59f42af4bedd5807af6d7f8901103827f5ff537))
* **brand:** round the app icon into a macOS-style squircle ([edee1be](https://github.com/kaiserbh/cathode/commit/edee1bec01018882369b1343e3f932ac51dc471c))
* **brand:** switch to the detailed retro-CRT app icon ([928306c](https://github.com/kaiserbh/cathode/commit/928306c9e1e413ef70ec718eff394b56ba95d9d8))
* browse Xtream live channels in the UI ([165ac9e](https://github.com/kaiserbh/cathode/commit/165ac9e02eb4423818de6386e54ee7eaf91ea36f))
* **browse:** hide content tabs a provider doesn't offer ([ff3ab8e](https://github.com/kaiserbh/cathode/commit/ff3ab8e58b382a625d26de78067ba9f05fb4fe92))
* **build:** implement macOS dylib placement for libmpv integration ([a44eaca](https://github.com/kaiserbh/cathode/commit/a44eaca10ccf32eb9779efbed9cd6421f38c0465))
* **catalog:** add Catalog trait, storage error, and Xtream source records ([9a0dbb0](https://github.com/kaiserbh/cathode/commit/9a0dbb05da25e0558c3eb2f010f2e5a490ee881b))
* **catalog:** add settings, favorites, and watch history to core ([e175df9](https://github.com/kaiserbh/cathode/commit/e175df90e03455887a454535e0c9148e811cebad))
* **catalog:** kind-keyed sqlite cache with ext and library search ([fa6f7d3](https://github.com/kaiserbh/cathode/commit/fa6f7d33029f4b8ad02fb603c05fefa188453e73))
* **catalog:** persist settings, favorites, and history in SQLite ([b626c11](https://github.com/kaiserbh/cathode/commit/b626c11e790e3b2d3eb8bdedac061d97f8cc7ed4))
* **catalog:** SQLite catalog with source and cache commands ([72dc0be](https://github.com/kaiserbh/cathode/commit/72dc0be1118ce991adec525fc1fe5c74dba1e30b))
* **channels:** add a List view alongside the Grid ([eea079d](https://github.com/kaiserbh/cathode/commit/eea079dff2b8cfd3f6206bc108110d67dc4d43c1))
* **commands:** kind-aware listing, series info, search, and playback ([e7c3c6c](https://github.com/kaiserbh/cathode/commit/e7c3c6c03b6217f78b4a80e4479589887fb3f4b3))
* **core:** add a structured LogLine record ([decf2d1](https://github.com/kaiserbh/cathode/commit/decf2d1810cfe7d450317c04cb253c498b6356bc))
* **core:** add container_extension and a series/episode model ([4315c70](https://github.com/kaiserbh/cathode/commit/4315c70df21a58ba80459b832c6b45bba2226a1b))
* **core:** add credential redaction and a persisted log-level setting ([2f13484](https://github.com/kaiserbh/cathode/commit/2f134849bb4cd8d02c4ebbf95522dbccc759ad94))
* **core:** add normalized model crate ([613547e](https://github.com/kaiserbh/cathode/commit/613547e3a6e20ef308ac27ee7ed2dd3cd4176f5f))
* **core:** fetch/parse VOD and series, kind-keyed catalog, and search ([69ac5f0](https://github.com/kaiserbh/cathode/commit/69ac5f02cff351446b77ed7100e7cfffbff2df9b))
* **core:** parse Xtream live streams; rename crate to cathode-core ([0043b57](https://github.com/kaiserbh/cathode/commit/0043b572387464e7e8151f7cbae8783ace381d6e))
* embedded mpv playback ([66db520](https://github.com/kaiserbh/cathode/commit/66db520f23f3cbe073f005c0956e7c26935c3df1))
* **epg:** add a timeline guide view mode ([f4e5425](https://github.com/kaiserbh/cathode/commit/f4e54252dd1657aff9071d12edbf9f2957bb49c6))
* **epg:** cache the full guide and serve name-aliased data ([131314f](https://github.com/kaiserbh/cathode/commit/131314f9ddc8c3b0910677ccd523071a051184cc))
* **epg:** fetch and cache the guide, expose epg_now_next ([7c35d41](https://github.com/kaiserbh/cathode/commit/7c35d41fd80e1f583f09f8c9bdf34ae02b7bf4ea))
* **epg:** match channels by name when they lack an epg_channel_id ([2bd6d25](https://github.com/kaiserbh/cathode/commit/2bd6d2582ae9cee9ecd4dbf059da5cd09813c24a))
* **epg:** parse channel display-names and add name matching ([52df20f](https://github.com/kaiserbh/cathode/commit/52df20f246c82fc503ea8f03f1e107005d14e44e))
* **epg:** parse XMLTV and match programmes to now/next ([80a68b2](https://github.com/kaiserbh/cathode/commit/80a68b2059ee87f47125e7dc24d3780b492b3989))
* **epg:** persist programmes in SQLite with descriptions ([4bcc5ed](https://github.com/kaiserbh/cathode/commit/4bcc5ed2e1703ac88cc3ace3c4cca710a5ab9fb3))
* **epg:** programme detail popover on cell click ([a238bab](https://github.com/kaiserbh/cathode/commit/a238babf6119df27aaa15b75f535f75e588b84e8))
* **epg:** show now/next on channel cards ([74ffba8](https://github.com/kaiserbh/cathode/commit/74ffba8110f21efc338f5909bcc128dd2c0d7c63))
* **epg:** virtualize guide rows with dx virtual_list ([953fd48](https://github.com/kaiserbh/cathode/commit/953fd4821b904bddc0041e9c330c9e8bda087bd3))
* fetch Xtream live data through Tauri commands ([cdf6cde](https://github.com/kaiserbh/cathode/commit/cdf6cde29bcd06209fb877b5c4dd70b25505564b))
* **library:** favorites, history, and Options tabs in the UI ([64e60f5](https://github.com/kaiserbh/cathode/commit/64e60f5da03d7167ea2e5a3b31aecf144020e288))
* **logs:** capture structured records scoped to Cathode's own crates ([c065750](https://github.com/kaiserbh/cathode/commit/c065750c273e21c5f17fd00719bbe6a67876828b))
* **logs:** capture tracing into a ring buffer with a live level switch ([c0bb517](https://github.com/kaiserbh/cathode/commit/c0bb5177686dd78129e7ca466d4e01538c841eb8))
* **logs:** give the logs modal a min height ([766bed4](https://github.com/kaiserbh/cathode/commit/766bed449042752e4b56da5081ab66e4fe87ebff))
* **logs:** keep event fields separate from the message ([818872b](https://github.com/kaiserbh/cathode/commit/818872b3aad6813982ba65db78b94ee925db15bf))
* **logs:** log fetch, guide, and playback outcomes so the panel has content ([2447920](https://github.com/kaiserbh/cathode/commit/2447920dcefa66e773ad6833fe3fde81871706e4))
* **logs:** make Trace capture everything and scope lower levels to Cathode ([96fb06b](https://github.com/kaiserbh/cathode/commit/96fb06b824a888f20a5b9c0febee39f159636632))
* **logs:** replace native select with primitives Select ([f6ef270](https://github.com/kaiserbh/cathode/commit/f6ef270ca0658dcbf184f66022fb374046edfd92))
* **playback:** render video in-window via macOS OpenGL surface ([223f5e3](https://github.com/kaiserbh/cathode/commit/223f5e3f799c0050456d5318efcb995901b6b802))
* **playback:** render video on Windows via ANGLE ([c6df98e](https://github.com/kaiserbh/cathode/commit/c6df98e3d75adc52a066c700c281919d14c4adb9))
* **player:** auto-hide controls and cursor on idle ([9f45d55](https://github.com/kaiserbh/cathode/commit/9f45d55d1890ed6ab7ba5a3f4c073cf9736872a6))
* **player:** backend volume, mute, and fullscreen controls ([0a5f3a4](https://github.com/kaiserbh/cathode/commit/0a5f3a4535ec6f0482ac57b912c57bf450308824))
* **player:** modern icon control bar with volume and QoL ([04b6183](https://github.com/kaiserbh/cathode/commit/04b6183e823e518c74c38927b86232fc27556303))
* **player:** show now/next in the player overlay ([d6749d9](https://github.com/kaiserbh/cathode/commit/d6749d9ed87f6f22934c4e58e14f003eb629a082))
* **player:** uniform volume slider via dioxus-primitives ([d24ad7d](https://github.com/kaiserbh/cathode/commit/d24ad7d015130040a66db5acd32a2fe81386b417))
* **player:** YouTube-style HUD for playback shortcuts ([e1e4a02](https://github.com/kaiserbh/cathode/commit/e1e4a02de70fe7bc42b57295c7f5597f0f4233ce))
* **settings:** make EPG and channel view customisable ([cdd50ef](https://github.com/kaiserbh/cathode/commit/cdd50efcbca17695eeb8e84ea1ce95742b8b1893))
* **sources:** multi-source browse UI with cache-first paint ([aa97998](https://github.com/kaiserbh/cathode/commit/aa97998eab476f694bafc9238066da1e1049b8b6))
* **ui:** add a Logs panel with level dropdown, copy, and clear ([e03eaf5](https://github.com/kaiserbh/cathode/commit/e03eaf534c49d0ae13c8a8032c500f0cea2297d6))
* **ui:** add clipboard helper and log bindings ([bbaf52f](https://github.com/kaiserbh/cathode/commit/bbaf52fac2b4940909d4ac5734a51b1e71f92b10))
* **ui:** add Dialog primitive and tune accent to sky ([8abeafc](https://github.com/kaiserbh/cathode/commit/8abeafc05a0ecbd6102b752cade054b602b0c7d1))
* **ui:** adopt Button for form and panel actions ([63e4310](https://github.com/kaiserbh/cathode/commit/63e43106e7a1a9dd2deb89a9bf2cd6d7f38f4b15))
* **ui:** adopt Switch for the toggle control ([8280e3a](https://github.com/kaiserbh/cathode/commit/8280e3a5f8e28256194936a8f205112908dbb44a))
* **ui:** adopt Tabs for the browse tab bar ([f21fada](https://github.com/kaiserbh/cathode/commit/f21fada05c7c941ae3941526eee61975817f2f7b))
* **ui:** adopt Toast provider/hook ([e7eecda](https://github.com/kaiserbh/cathode/commit/e7eecda109150e31977be83fb932049d4d1aad90))
* **ui:** back the icon set with Lucide via dioxus-icons ([8db36d5](https://github.com/kaiserbh/cathode/commit/8db36d5f164eadcc9f423d48e2e7e01c40c51591))
* **ui:** color log rows and add level + search filtering ([720c498](https://github.com/kaiserbh/cathode/commit/720c498ee5fde836683f2d962fffbe99b5b64cde))
* **ui:** fold title and controls into a draggable titlebar with icon buttons ([e48c8f0](https://github.com/kaiserbh/cathode/commit/e48c8f0a1d8e730ff7b816b346f871daf01b3fcb))
* **ui:** increase log panels min height. ([5cffbcd](https://github.com/kaiserbh/cathode/commit/5cffbcd0864c17a09bc3adddb716e853e604e7ee))
* **ui:** move modals onto the dioxus-primitives Dialog ([fb0e749](https://github.com/kaiserbh/cathode/commit/fb0e749ac3f0536d924cc44c09973d34a3f5ab24))
* **ui:** Movies/Series tabs, series drill-down, and library search ([027d722](https://github.com/kaiserbh/cathode/commit/027d722c3ed88d310083609a62c1035263d73e38))
* **ui:** open the Logs panel from a titlebar bug icon ([1c4e52f](https://github.com/kaiserbh/cathode/commit/1c4e52fd11879d4335286e0782d6f50d83949362))
* **ui:** render colored, aligned, auto-scrolling logs with icon actions ([aec44ba](https://github.com/kaiserbh/cathode/commit/aec44ba3184d99a9593c959c454dac54a8db6d90))
* **ui:** show a toast when logs are copied ([7f5196c](https://github.com/kaiserbh/cathode/commit/7f5196c27edc22d3748272f1353f59c072d067a9))
* **ui:** title-bar tooltips + draggable top chrome ([9eadabc](https://github.com/kaiserbh/cathode/commit/9eadabca7dd44069f2cb717a8cfc8e97ddbf3228))
* **ui:** use Lucide stars and history icon for favorites/tabs ([c265e22](https://github.com/kaiserbh/cathode/commit/c265e2200261027d15c32b132328a14da2dcc941))
* **ui:** vendor slider + select primitives ([06dc7e5](https://github.com/kaiserbh/cathode/commit/06dc7e5b5cee76ad46dd9dc30563ac099d60aa46))
* **ui:** vendor Switch/Tabs/Toast/Tooltip primitives ([5ddf938](https://github.com/kaiserbh/cathode/commit/5ddf938db0b5ba821b2c226d8862ca83d9c090ba))
* **window:** capitalize the app name and set default + minimum size ([e4fd009](https://github.com/kaiserbh/cathode/commit/e4fd0095e193a432f56d57b20e2154e45378f503))


### Bug Fixes

* **brand:** give the app icon real transparency ([7f867a5](https://github.com/kaiserbh/cathode/commit/7f867a56e94b897cafc4bd05f322be619cf4567c))
* **browse:** drop stale category responses and scroll panes independently ([bdd4551](https://github.com/kaiserbh/cathode/commit/bdd45511f466ad2d640629bdc29c1b5e720c763e))
* dragging window ([f182e11](https://github.com/kaiserbh/cathode/commit/f182e117d7c8efd7c2bd9a77d2144ae206e0a97e))
* **epg:** bound the virtualized guide height ([d922d32](https://github.com/kaiserbh/cathode/commit/d922d320c4a9d84a3fe2492dcfec0b030548e64a))
* **epg:** make the programme popover title readable ([48cebe4](https://github.com/kaiserbh/cathode/commit/48cebe4fccfdca1c8e293d52309354243b007e4d))
* **http:** redact secrets in network errors and set a User-Agent ([02f3d3d](https://github.com/kaiserbh/cathode/commit/02f3d3d89d48f6d85bdcfa40ded3e45d29f33899))
* **logs:** reflect the chosen level in the dropdown ([e34cd82](https://github.com/kaiserbh/cathode/commit/e34cd8201f6c4203f799624702c3dccc4c9e5fbb))
* **playback:** volume control fix ([1706aac](https://github.com/kaiserbh/cathode/commit/1706aac983c0579684c0691fff32f668ef3c908e))
* **player:** constrain volume slider so it stops eating control clicks ([75477cc](https://github.com/kaiserbh/cathode/commit/75477cc80547eb03059269b5ba458b09b3d1b2db))
* **player:** fill the volume slider track up to the thumb ([139984a](https://github.com/kaiserbh/cathode/commit/139984aece151152dcab5fbaf102bae5552391ba))
* **player:** recapture overlay focus after control use so shortcuts keep working ([686b461](https://github.com/kaiserbh/cathode/commit/686b461ee4f2688279b842f3c8de798bad4888b9))
* **player:** replace dx Slider with a styled native range ([8e45429](https://github.com/kaiserbh/cathode/commit/8e45429d1c099e08dbdd4f963b68dbce225a25c6))
* **player:** show a centered spinner and drop the refreshing hint ([6a3f9b8](https://github.com/kaiserbh/cathode/commit/6a3f9b85f04e990d616dea56566a0ace2a4fd006))
* **player:** steady, more-transparent shortcut HUD ([ae1da50](https://github.com/kaiserbh/cathode/commit/ae1da50efa251af71e7e55c3eb7cedc0aa5710dd))
* **ui:** keep dialogs off the screen edges and responsive ([de35927](https://github.com/kaiserbh/cathode/commit/de35927c6ba4ac9bb845136336bfb0ad06c5f5f9))
* **ui:** keep the logs search input readable in dark mode ([f419ad2](https://github.com/kaiserbh/cathode/commit/f419ad2d2d2205b0242a7fe87121e91239b2a300))
* **ui:** let panel width control dialog size ([4289698](https://github.com/kaiserbh/cathode/commit/42896989269fb56eee19740152adf7e74596d303))
* **ui:** make search results readable in dark mode and center the empty state ([6a81989](https://github.com/kaiserbh/cathode/commit/6a8198941cf43e7ce1dfae1e29df6789445eb4d6))
* **ui:** make titlebar draggable on macOS (spacer + startDragging fallback) ([9fc5cb0](https://github.com/kaiserbh/cathode/commit/9fc5cb05a3565496000a1262b6bd9d16c2f96200))
* **ui:** stop modal focus trap from freezing the app ([956c6a0](https://github.com/kaiserbh/cathode/commit/956c6a03442d0d1bebdfb55f23896e4c5f8bb3a8))
* **ui:** widen titlebar drag zone to w-28 (112 px) ([b2e0e7c](https://github.com/kaiserbh/cathode/commit/b2e0e7cb74a41e0ea48608ffa563d38e18b9909c))


### Miscellaneous Chores

* set initial release to 0.5.0 ([841c19f](https://github.com/kaiserbh/cathode/commit/841c19ffac0836df507031291bc312525c5ad84b))
