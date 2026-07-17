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

## Baad mein fix kiye gaye 2 aur Android-specific bugs

### 1. `i`/`I` (incognito open) sirf Chrome khol raha tha, URL load nahi kar raha tha, incognito bhi on nahi ho raha tha

**Asli wajah:** `platform.rs` pehle Chrome ke `EXTRA_OPEN_NEW_INCOGNITO_TAB` intent extra ka
use kar raha tha (wahi wala jo ADB one-liners mein aam dikhta hai). Chrome ye extra
**jaan-boojh kar ignore kar deta hai** jab intent kisi aise app se aaye jo Google se
sign nahi hai — Termux definitely Google-signed nahi hai. Isliye Chrome bas normally
khul jaata tha, na URL pe navigate karta tha na incognito on hota tha.

**Fix:** Ab `org.chromium.chrome.browser.incognito.OPEN_PRIVATE_TAB` action use hota
hai, jo **kisi bhi app** ke liye publicly available hai (koi signature check nahi).
Lekin — ye bhi jaan-boojh kar (privacy ke liye) **URL accept nahi karta**, sirf ek
blank incognito tab khol sakta hai. Isliye ab flow ye hai:
1. URL (ya, `I` ke case mein, poori list ek-ek line mein) clipboard pe copy hoti hai
2. Ek blank incognito tab khulta hai
3. TUI mein ek info message dikhta hai jo bata deta hai ki paste karna hai

Ye Android/Chrome ki genuine platform-level restriction hai (koi third-party app URL
seedha incognito mein load nahi karwa sakta), na ki bmm ka koi bug — isliye ye best
possible fix hai is limitation ke andar rehte hue.

### 2. Mobile pe `Y` (list copy) kaam nahi kar raha tha, sirf `y` (single copy) kaam kar raha tha

**Asli wajah:** Kuch Android soft-keyboards/terminal setups Shift+Y ko uppercase
`'Y'` character ki jagah lowercase `'y'` + `SHIFT` modifier flag ke roop mein bhejte
hain. Desktop/hardware keyboards par ye issue nahi aata kyunki wahan terminal khud hi
Shift ko uppercase character mein convert kar deta hai crossterm tak pahunchne se
pehle.

**Fix:** `src/tui/message.rs` mein ab `Char('y')` + `SHIFT` modifier wale combination
ko bhi `CopyURIsToClipboard` (wahi jo `Y` karta hai) maana jaata hai, `Char('Y')` wale
match ke saath-saath. List copy pehle se hi ek link per line clipboard pe jaati thi
(`uris.join("\n")`), wo waisa hi hai.

### Note on testing

Ye do fixes bhi maine real Termux device pe test nahi kiye (upar wajah dekhein — is
sandbox mein Rust 1.85+ install nahi ho paaya). Logic maine bahut carefully review
kiya hai aur upar bataye gaye root causes verified/well-documented hain (Chromium ke
source code discussion se confirm kiya), lekin **ek baar apne Termux pe build karke
zaroor test karein**. Agar `am start -a org.chromium.chrome.browser.incognito.OPEN_PRIVATE_TAB`
wala command directly terminal mein chala ke dekhein to bhi kaam karna chahiye:

```bash
am start -a org.chromium.chrome.browser.incognito.OPEN_PRIVATE_TAB -n com.android.chrome/com.google.android.apps.chrome.Main
```

Agar aapka default browser Chrome nahi hai (ya package name `com.android.chrome` nahi
hai), to ye command fail hoga — is case mein bata dena, alag logic likhna padega.

