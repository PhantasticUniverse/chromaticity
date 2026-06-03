#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# release-eng/dev-run-macos.sh — build, sign, and launch the Chromaticity
# desktop app locally on macOS with a STABLE self-signed identity, so that
# macOS TCC (microphone / system-audio capture) consent PERSISTS across
# rebuilds.
#
# Why this exists
# ---------------
# A bare `cargo run` binary — or an ad-hoc `codesign -s -` — gets a fresh
# code-hash on every build, so macOS treats each rebuild as a new app and
# RE-shows the audio-capture consent prompt every single time. That per-build
# re-prompt is what turned system-audio debugging into a marathon (every fix
# needed a human to click "Allow" again). Signing with a stable self-signed
# certificate gives the bundle a stable "designated requirement", so consent is
# granted ONCE and survives rebuilds — enabling autonomous hardware iteration
# (build → relaunch → drive audio → read diagnostics, no human in the loop).
#
# This is a DEV tool, NOT the release path (that is `sign-macos.sh` +
# `notarize.sh` with the real Developer ID cert). The self-signed cert lives in
# a DEDICATED keychain with a known password, so the whole flow is
# non-interactive (no login-keychain password prompts). The only prompt you
# will ever see is the one-time macOS audio-capture consent.
#
# Usage
# -----
#   release-eng/dev-run-macos.sh [--diag] [--device <sentinel>] [--no-build]
#                                [--log <path>] [--tone] [-h]
#
#   --diag            launch with CHROMATICITY_AUDIO_DIAG=1 (dBFS telemetry to
#                     stderr -> the --log file); see the run/test guide.
#   --device <s>      set settings.audio.input_device before launch. Common
#                     sentinels: `default` (host default input / mic),
#                     `system-audio` (zero-install output tap). Applied on
#                     restart only.
#   --no-build        skip `npm run build` + `cargo build` (reuse the last
#                     binary; just re-sign + relaunch).
#   --log <path>      diagnostics stderr log (default: /tmp/chroma-diag.log).
#   --tone            after launch, play a known 440 Hz / 0.5-amplitude sine
#                     through the system output (the authoritative capture probe)
#                     and tail the diag log. Implies --diag.
#   -h | --help       print this header.
#
# See docs/dev/macos-desktop-run-and-audio-diagnostics.md for the full guide.

set -euo pipefail

# ---------------------------------------------------------------- config -----
BUNDLE_ID="app.chromaticity.desktop"
APP_NAME="Chromaticity"
EXECUTABLE="chromaticity-app"
CERT_CN="Chromaticity Dev (self-signed)"
DEV_KEYCHAIN="$HOME/Library/Keychains/chromaticity-dev.keychain-db"
DEV_KEYCHAIN_PW="chromaticity-dev"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_SRC="$REPO_ROOT/target/debug/$EXECUTABLE"
ENTITLEMENTS="$REPO_ROOT/release-eng/entitlements.plist"
APP_DIR="${CHROMA_DEV_APP_DIR:-$REPO_ROOT/target/dev-macos/$APP_NAME.app}"
SETTINGS_TOML="$HOME/Library/Application Support/$BUNDLE_ID/settings.toml"

# ---------------------------------------------------------------- flags ------
DIAG=0; BUILD=1; DEVICE=""; DIAG_LOG="/tmp/chroma-diag.log"; TONE=0
while [ $# -gt 0 ]; do
  case "$1" in
    --diag) DIAG=1 ;;
    --no-build) BUILD=0 ;;
    --device) DEVICE="${2:?--device needs a value}"; shift ;;
    --log) DIAG_LOG="${2:?--log needs a value}"; shift ;;
    --tone) TONE=1; DIAG=1 ;;
    -h|--help) grep '^#' "$0" | sed 's/^#\{1,\} \{0,1\}//'; exit 0 ;;
    *) echo "unknown arg: $1 (try --help)" >&2; exit 2 ;;
  esac
  shift
done

log() { printf '\033[36m[dev-run]\033[0m %s\n' "$*"; }

if [ "$(uname -s)" != "Darwin" ]; then
  echo "dev-run-macos.sh is macOS-only (this host is $(uname -s))." >&2
  exit 1
fi

# ----------------------------------------------- 1. stable signing identity --
# Create a dedicated keychain holding a self-signed code-signing cert. Idempotent:
# re-running reuses the existing cert (that stability is the whole point — it
# keeps the designated requirement, hence TCC consent, constant across rebuilds).
ensure_identity() {
  if security find-identity -p codesigning "$DEV_KEYCHAIN" 2>/dev/null | grep -qF "$CERT_CN"; then
    log "reusing stable signing identity: $CERT_CN"
    return 0
  fi
  log "creating one-time stable signing identity: $CERT_CN"
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN

  cat > "$tmp/cert.cnf" <<EOF
[req]
distinguished_name = dn
x509_extensions    = v3
prompt             = no
[dn]
CN = $CERT_CN
[v3]
keyUsage          = critical, digitalSignature
extendedKeyUsage  = critical, codeSigning
basicConstraints  = critical, CA:false
EOF

  openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
    -keyout "$tmp/key.pem" -out "$tmp/cert.pem" -config "$tmp/cert.cnf" >/dev/null 2>&1
  openssl pkcs12 -export -inkey "$tmp/key.pem" -in "$tmp/cert.pem" \
    -out "$tmp/cert.p12" -passout pass:"$DEV_KEYCHAIN_PW" -name "$CERT_CN" >/dev/null 2>&1

  # Dedicated keychain so we own the password -> no login-keychain GUI prompts.
  security create-keychain -p "$DEV_KEYCHAIN_PW" "$DEV_KEYCHAIN" 2>/dev/null || true
  security set-keychain-settings "$DEV_KEYCHAIN"           # no auto-lock timeout
  security unlock-keychain -p "$DEV_KEYCHAIN_PW" "$DEV_KEYCHAIN"
  security import "$tmp/cert.p12" -k "$DEV_KEYCHAIN" -P "$DEV_KEYCHAIN_PW" \
    -T /usr/bin/codesign -A
  # Let codesign use the key without an interactive ACL prompt (we know the pw).
  security set-key-partition-list -S apple-tool:,apple: -s -k "$DEV_KEYCHAIN_PW" \
    "$DEV_KEYCHAIN" >/dev/null 2>&1 || true
  # Add to the user search list so `codesign --sign "<CN>"` resolves it.
  local existing; existing="$(security list-keychains -d user | sed 's/[" ]//g' | tr '\n' ' ')"
  # shellcheck disable=SC2086
  security list-keychains -d user -s "$DEV_KEYCHAIN" $existing
  log "identity created (dedicated keychain: $DEV_KEYCHAIN)"
}

# --------------------------------------------------------------- 2. build ----
build() {
  [ "$BUILD" -eq 1 ] || { log "skipping build (--no-build)"; return 0; }
  log "npm run build (frontend)…"
  ( cd "$REPO_ROOT" && npm run build >/dev/null )
  # vite build clobbers the load-bearing dist sentinel; restore it.
  ( cd "$REPO_ROOT" && { git checkout -- dist/.gitkeep dist/help 2>/dev/null || true; } )
  log "cargo build -p chromaticity-app…"
  ( cd "$REPO_ROOT" && cargo build -p chromaticity-app )
}

# ---------------------------------------------------- 3. bundle + sign -------
bundle_and_sign() {
  [ -f "$BIN_SRC" ] || { echo "binary not found: $BIN_SRC (run without --no-build)" >&2; exit 1; }
  log "assembling $APP_DIR"
  rm -rf "$APP_DIR"
  mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
  cp "$BIN_SRC" "$APP_DIR/Contents/MacOS/$EXECUTABLE"

  cat > "$APP_DIR/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
  <key>CFBundleExecutable</key><string>$EXECUTABLE</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>0.0.0</string>
  <key>CFBundleShortVersionString</key><string>0.0.0</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSMicrophoneUsageDescription</key>
  <string>Chromaticity reacts to live audio input (microphone or selected device).</string>
  <key>NSAudioCaptureUsageDescription</key>
  <string>Chromaticity captures system audio output to drive its visuals.</string>
</dict>
</plist>
EOF

  # Entitlements: strip HTML/XML comments the codesign parser rejects.
  local ent="/tmp/chroma-dev-ent.plist"
  if [ -f "$ENTITLEMENTS" ]; then
    plutil -convert xml1 "$ENTITLEMENTS" -o "$ent" 2>/dev/null || cp "$ENTITLEMENTS" "$ent"
  else
    ent=""
  fi

  log "codesign with stable identity"
  if [ -n "$ent" ]; then
    codesign --force --sign "$CERT_CN" --keychain "$DEV_KEYCHAIN" \
      --identifier "$BUNDLE_ID" --entitlements "$ent" \
      "$APP_DIR/Contents/MacOS/$EXECUTABLE"
  else
    codesign --force --sign "$CERT_CN" --keychain "$DEV_KEYCHAIN" \
      --identifier "$BUNDLE_ID" "$APP_DIR/Contents/MacOS/$EXECUTABLE"
  fi
  codesign --force --deep --sign "$CERT_CN" --keychain "$DEV_KEYCHAIN" \
    --identifier "$BUNDLE_ID" "$APP_DIR" 2>/dev/null || true
}

# ---------------------------------------------------- 4. device preference ---
apply_device() {
  [ -n "$DEVICE" ] || return 0
  if [ -f "$SETTINGS_TOML" ] && grep -q '^input_device' "$SETTINGS_TOML"; then
    sed -i '' "s/^input_device = .*/input_device = \"$DEVICE\"/" "$SETTINGS_TOML"
    log "set input_device = \"$DEVICE\" in settings.toml"
  else
    log "note: settings.toml has no input_device line yet; pick the device in Settings → Audio (it persists)."
  fi
}

# --------------------------------------------------------------- 5. launch ---
launch() {
  osascript -e "quit app \"$APP_NAME\"" 2>/dev/null || true
  sleep 1
  pkill -f "$APP_DIR/Contents/MacOS/$EXECUTABLE" 2>/dev/null || true
  sleep 1
  : > "$DIAG_LOG"
  local args=(open "$APP_DIR")
  if [ "$DIAG" -eq 1 ]; then
    args=(open --env CHROMATICITY_AUDIO_DIAG=1 --stderr "$DIAG_LOG" "$APP_DIR")
    log "launching with CHROMATICITY_AUDIO_DIAG=1 -> $DIAG_LOG"
  else
    log "launching $APP_NAME"
  fi
  "${args[@]}"
}

# --------------------------------------------------- 6. optional tone probe --
tone_probe() {
  [ "$TONE" -eq 1 ] || return 0
  log "waiting for capture to start (grant the consent prompt if it appears)…"
  local ok=0
  for _ in $(seq 1 120); do
    if grep -qE "audio-diag.*cb=[1-9]" "$DIAG_LOG" 2>/dev/null; then ok=1; break; fi
    sleep 1
  done
  [ "$ok" -eq 1 ] || { log "no callbacks yet — still blocked on consent, or no capture. Check $DIAG_LOG."; return 0; }
  log "playing 440 Hz / 0.5-amplitude sine (the authoritative capture probe)…"
  osascript -e 'set volume output volume 100' 2>/dev/null || true
  python3 - <<'PY'
import wave, struct, math
sr=44100; dur=12; amp=0.5; f=440.0
w=wave.open('/tmp/chroma-tone.wav','wb'); w.setnchannels(2); w.setsampwidth(2); w.setframerate(sr)
b=bytearray()
for i in range(sr*dur):
    v=int(amp*math.sin(2*math.pi*f*i/sr)*32767); b+=struct.pack('<hh', v, v)
w.writeframes(bytes(b)); w.close()
PY
  afplay /tmp/chroma-tone.wav &
  sleep 13
  log "=== engine readout during the tone (expect rms ~ -9 dBFS, peak ~ -6, a 440 Hz FFT spike) ==="
  grep "audio-diag.*cb=" "$DIAG_LOG" | tail -4 || true
  log "PASS = peak <= 0.5, crest ~3 dB, single 440 Hz bin, rms ~ -9 dBFS. (See ADR-022 Amendment 2026-06-03.)"
}

ensure_identity
build
bundle_and_sign
apply_device
launch
tone_probe
log "done. App: $APP_DIR  ·  diag log: $DIAG_LOG"
