# WristVault

**Secure recovery codes stored on your Timex Datalink 150**

WristVault is a wristapp for the Timex Datalink 150 watch that stores emergency recovery codes for your critical online accounts fully in RAM. It's designed as a last-resort backup when traveling — if you lose access to your password manager or security keys (like YubiKeys), you can use your watch to restore account access.

## ⚠️ Use Case

This project is designed for individuals who:

- Travel often and want an ultra-portable, last-resort recovery option,
- Understand the limitations and benefits of the Timex Datalink 150 hardware,
- Are willing to trade convenience for security for extremely rare scenarios.

## 🔐 Security Design

- **RAM-based storage**: The recovery codes are embedded directly in the wristapp and stored only in volatile memory (RAM).
- **No persistent storage**: Timex Datalink's architecture does not allow reading RAM externally without overwriting the app itself.
- **Tamper resistance**:
  - A custom password (set of keypresses) is required to unlock the codes.
  - Too many wrong attempts will auto-wipe the recovery codes.
- **Factory reset protection**: Resetting the watch (pressing all buttons for 3 seconds) securely erases the codes.

## 🕹️ Features

- 💾 Embedded recovery codes
- 🔒 Unlock via keypress password
- 🧼 Auto-wipe upon brute-force attempts
- ⚡ Fast emergency reset (hardware-level, 3-button hold)
- 🕰️ Offline and untrackable — completely air-gapped

## 🔐 Threat Model

- **Attacker with physical access**: Cannot read stored codes without overwriting the app — which erases them.
- **Accidental factory reset**: Deletes codes — only use if you're sure you're safe.
- **Brute-force attack prevention**: Limited password attempts before self-wipe.

## 🛠️ Setup Instructions

<!-- TODO -->

