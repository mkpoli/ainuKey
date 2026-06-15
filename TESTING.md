# Testing ainuKey on Windows

Everything past v0.1.0 compiles (`cargo check --target x86_64-pc-windows-msvc`)
but needs a real Windows smoke test — pixels, focus, registration, and the
`InstallLayoutOrTip` path can't be verified by the compiler. This is the
checklist. The branches stack, so test in order; for each, `.\build.ps1` then
`.\install.ps1` (UAC), and `.\install.ps1 -Uninstall` to back out.

## Prerequisites (once)
- [Rust](https://rustup.rs) + MSVC toolchain + Windows SDK ("Desktop development
  with C++" in the VS installer).
- `rustup target add x86_64-pc-windows-msvc`

## PR #1 — v0.1.0 (`rust-migration`): core input ⟵ start here (rights-clean)
- [ ] **ainuKey appears** in the input switcher (Win+Space). In v0.1 it's under
      the Japanese group — add Japanese under Settings → Language if absent.
- [ ] Notepad: type `kamuy` → underlined preedit shows カムイ → **Space commits** カムイ.
- [ ] Forgiving input: `ti` → チ; `chise` → チセ; `aynu` → アイヌ.
- [ ] Backspace edits the preedit; Esc cancels.
- [ ] Works in a **UWP** app too (e.g. the Settings search box) — not just Win32.
- If not in the switcher → confirm `regsvr32` succeeded (elevated). Sanity:
  `dumpbin /exports ainukey.dll` should list the four `Dll*` entries.

## PR #2 — v0.2 Ainu locale (`v0.2-ainu-locale`)
`install.ps1` here also runs the per-user enable (`Set-WinUserLanguageList` +
`InstallLayoutOrTip`).
- [ ] **"Ainu"** (not Japanese) shows in the switcher.
- [ ] Error "no transient LCID assigned" → all four transient slots
      (`0x2000/0x2400/0x2800/0x2C00`) are in use by other IMEs; free one and retry.
- [ ] Uninstall removes the language and disables the TIP.

## PR #4 — v0.2 mode-switch + settings (`v0.2-mode-ui`)
- [ ] A **language-bar button** appears (ア / A icon).
- [ ] Clicking toggles **kana ⇄ Latin**; in Latin mode letters type through raw
      (no composition).
- [ ] Language settings → ainuKey → **Options** opens the settings dialog.

## PR #5 — v0.3 suggestions (`v0.3-candidate`) — RIGHTS-GATED, local only
Requires a locally generated table: `uv run tools/build_ngrams.py` (needs the
private `ainu-corpora`). Public builds have suggestions disabled by design.
- [ ] A **candidate popup** appears near the caret while typing.
- [ ] **↑/↓** navigate, **digits 1–9** pick, **Space** commits the highlighted one.
- [ ] After committing a word, the next word's completions are **context-ranked**
      (e.g. after `a=kor`, typing `o` surfaces `ona`).

## Reporting back
For any ✗, paste the symptom and any output. Most-likely first issues, in order:
1. **Candidate-window position/visibility** (`GetGUIThreadInfo` caret lookup, GDI paint).
2. **`InstallLayoutOrTip` install-string format** (`%04x:{CLSID}{PROFILE}` — Keyman
   style, no `0x`; MS docs show a `0x` variant — this is the top thing to confirm).
3. **Edit-session sync** rejected by some hosts (`TF_E_SYNCHRONOUS`) → needs an
   async fallback.
4. The display-attribute **underline** not rendering.
