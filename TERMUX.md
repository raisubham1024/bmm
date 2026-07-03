# bmm on Termux — kya fix kiya gaya, aur kaise build/test karna hai

## Asli problem kya thi

`bmm` do desktop-only crates use karta tha:

1. **`arboard`** (clipboard) — Linux/Mac/Windows pe X11/Wayland/Win32/Cocoa
   windowing system chahiye hota hai. Termux/Android me woh hota hi nahi, isliye
   `arboard` ki X11 backend hi **compile fail** ho jaati thi.
2. **`open`** — browser me URL kholne ke liye `xdg-open` jaisi cheezein
   dhoondhta hai, jo Termux me exist nahi karti (runtime error/silently fail).

## Fix kya kiya

Naya module `src/platform.rs` banaya jo compile-time pe decide karta hai
(`#[cfg(target_os = "android")]`) ki kaunsa backend use karna hai:

- **Non-Android (Linux/Mac/Windows)**: bilkul pehle jaisa hi — `arboard` +
  `open` crate. Koi behavior change nahi.
- **Android/Termux**: `arboard`/`open` Cargo.toml me **dependency hi nahi
  hai** ab (`[target.'cfg(not(target_os = "android"))'.dependencies]` ke
  andar), isliye Termux pe X11 headers dhoondhne ki zaroorat hi nahi padegi —
  build clean hoga. Iske bajaye do commands shell out karte hain:
  - Copy → `termux-clipboard-set`
  - Open URL → `termux-open-url`

  Ye dono commands **Termux:API** app (F-Droid/Play Store) + `termux-api`
  package se aate hain.

Sirf 4 files touch hue: `Cargo.toml`, `src/main.rs`, `src/tui/handle.rs`, aur
naya `src/platform.rs`. Baaki poora codebase (DB, search, TUI, CLI) waisa hi
hai jaisa upstream `dhth/bmm` me hai.

## Termux pe setup + build

```bash
pkg update && pkg upgrade
pkg install rust termux-api sqlite

# Termux:API app bhi Play Store/F-Droid se install karna zaroori hai
# (sirf `pkg install termux-api` command-line tools deta hai, actual
# clipboard/open functionality is Android app ke through kaam karti hai)

git clone <aapke-fork-ka-url> bmm
cd bmm
cargo build --release

# binary yahan milegi:
./target/release/bmm --help
```

## Verify karo ki fix kaam kar raha hai

```bash
# clipboard test
echo "hello" | termux-clipboard-set   # ye direct command hai, bmm ke bina
termux-clipboard-get                  # "hello" print hona chahiye

# bmm ke andar se test:
bmm save https://example.com --title "test"
# TUI kholo (bmm), bookmark select karo, 'y' (ya jo bhi keybind ho copy ke liye)
# dabao, phir kisi aur app me paste karke check karo
```

Agar `termux-clipboard-set`/`termux-open-url` command hi na mile, `bmm` ab
crash nahi karega — clear error dega ("termux-api command not found... pkg
install termux-api") jisse pata chal jaayega exactly kya missing hai.

## Note

Ye sandbox environment me maine Rust 1.75 (Ubuntu apt) se hi verify kiya —
`edition = "2024"` (jo ye repo use karta hai) ke liye Rust >= 1.85 chahiye,
jo is sandbox me install nahi ho paaya (rustup ka domain network policy me
allowed nahi hai). Maine code **manually carefully review** kiya hai aur
logic straightforward hai (bas process spawn + stdin write), lekin **aapko
apne Termux/Linux machine pe ek baar `cargo build` chalana chahiye** taaki
confirm ho jaaye. Agar koi compile error aaye to woh mujhe paste kar dena,
turant fix kar dunga.
