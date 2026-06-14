# ainuKey

> [!NOTE]
> <span lang="ja">早期ベータ版です。基本的なローマ字→カタカナ入力が利用できます。</span>
>
> Early beta. Basic romaji → Ainu katakana input works; more features are on the way.

Iokere ipak / 進捗 / Progress : ![Progress](https://progress-bar.xyz/25/?width=200)

## <span lang="ain">Nep kusu</span> / <span lang="ja">概要</span> / Overview
<p lang="ain">
    Tan porokram anakne Windows or ta Aynuitak ani aeynuyep ne.
</p>

<p lang="ja">
    このソフトはWindowsでアイヌ語を入力するためのIMEです。
</p>

<p>
    This software is an IME for the Ainu language on Windows. It is a Text
    Services Framework (TSF) text input processor written in Rust.
</p>

## <span lang="ja">使い方</span> / Usage

Type Latin romanization and the in-progress text appears underlined as a
composition. Press <kbd>Space</kbd> or <kbd>Enter</kbd> to convert it to Ainu
katakana and commit; <kbd>Backspace</kbd> edits, <kbd>Esc</kbd> cancels. Input
is forgiving — e.g. `ti` becomes `ci` (チ). Conversion is powered by
[ainconv](https://github.com/mkpoli/ainconv-rs).

## <span lang="ja">インストール</span> / Install

From the [Releases](https://github.com/mkpoli/ainuKey/releases) page:

- **Installer (recommended):** download `ainuKey-vX.Y.Z-x86_64-windows-setup.exe`
  and run it. It installs to `C:\Program Files\ainuKey` and registers the input
  method; uninstall from **Settings → Apps**.
- **Portable zip:** download `…-x86_64-windows-msvc.zip`, unzip, right-click
  `install.ps1` → **Run with PowerShell** (self-elevates). Remove with
  `install.ps1 -Uninstall`.

Then switch input method (<kbd>Win</kbd>+<kbd>Space</kbd>) to **ainuKey**.

(The installer is unsigned, so SmartScreen may warn — choose "More info → Run anyway".)

> v0.1 registers under the `ja-JP` langid, so Japanese may need to be added under
> **Settings → Time & language → Language** for ainuKey to appear in the switcher.

## <span lang="ja">ビルド</span> / Build from source

Requires [Rust](https://rustup.rs) and the MSVC toolchain + Windows SDK
("Desktop development with C++").

```powershell
.\build.ps1     # cargo build --release --target x86_64-pc-windows-msvc
.\install.ps1   # install + register (elevated)
```

## A=kar / 開発 / Development

https://zenn.dev/mkpoli/scraps/6dc57fcd0335cf

## License

MIT © 2024–2026 mkpoli — see [LICENSE](LICENSE).
