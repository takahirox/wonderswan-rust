# Issue #9 post-merge verification

Performed on 2026-10-20 after merging [PR #10](https://github.com/takahirox/wonderswan-rust/pull/10).

- Source Issue #9 was read immediately before the merge and confirmed open.
- GitHub merged with the required head SHA `dbff640b17a067850fa8c57f67d56ac13541f682`
  and merge method `merge`, preserving checkpoint history. No protection bypass
  was requested.
- Confirmed merge commit: `6a5c8798d061035c171a00d8daf3c3eddcae7040`.
- [Pages workflow run 39406180454](https://github.com/takahirox/wonderswan-rust/actions/runs/39406180454)
  completed successfully for that commit on `main`, including Wasm compilation,
  frontend build, artifact upload, and Pages deployment.

## Public player

Opened <https://takahirox.github.io/wonderswan-rust/> in fresh, unauthenticated
headless Chromium contexts. Installed existing locked frontend dependencies with
`npm ci` and ran from `web`:

```sh
WS_PREVIEW_URL=https://takahirox.github.io/wonderswan-rust/ \
WS_SMOKE_ARTIFACTS=/tmp/wonderswan-issue-9-production npm run test:browser
```

Result: **passed**, exit code 0. This exercised the deployed site, not a local preview:

- Bundled demo boot, completion, and Start restart on SwanCrystal, WonderSwan,
  and WonderSwan Color; model switching, pause/resume, and UI restart.
- Local ROM playback without uploads, invalid-file recovery, and execution-error
  recovery.
- Settings focus, Escape, input isolation, conditional resume, and Debug Mode
  default/off/on behavior with factual diagnostics.
- Native Chromium fullscreen entry/exit, correct aspect ratio, and pixelated
  rendering. Simulated unsupported/rejected fullscreen cases also passed.
- Initialization/loading/retry and asynchronous modal/background recovery.
- Desktop 1440×900 and mobile 375×812 layouts; touch direction/A/B/Start input,
  holding, release, cancellation, blur, and mobile settings/debug behavior.
- Production Wasm/ROM requests, CC0 notice, GET-only normal requests, and no
  unexpected browser errors.

Separately followed all four Settings links (demo ROM, editable source, provenance
notice, full CC0 license) and fetched deployed JS/CSS assets: all returned HTTP 200.
Reviewed the production desktop and mobile playing screenshots: the game remains
dominant, pixels retain their aspect ratio, and primary controls are legible and
unclipped without horizontal overflow. No deployment-only defects were observed.

Fault injection and visibility changes are browser-local simulations. These
results do not establish additional-browser or physical-touch-device coverage.
Both required post-merge checks are complete. This record is saved in the merge
node's checkpoint; it does not change the already merged PR head.
