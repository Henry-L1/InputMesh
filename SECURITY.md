# Security Policy

## Supported versions

InputMesh is currently a preview. Security fixes are applied to the latest
release and the `main` branch only.

## Threat model

InputMesh treats LAN discovery as untrusted. A discovered device receives no input until both endpoints complete a Noise XX handshake, compare the same short authentication string, and explicitly approve the peer. On later connections, the stored static public key must match.

The project does not attempt to protect a computer after its operating system or the InputMesh process has been compromised. It also does not cross the Windows secure desktop or macOS login-window boundary.

## Operational guidance

- Pair only devices you control and compare the code on both physical displays.
- Enable the app only on trusted private networks.
- Revoke peers you no longer use.
- Treat the local identity/config file as sensitive; it contains a private Noise key.
- Keep the emergency shortcut `Ctrl + Alt + Esc` available.

## Reporting a vulnerability

Do not publish exploit details in a public issue. Use GitHub's private
vulnerability reporting form from the repository's **Security** tab. Include
the affected version, platform, impact and minimal reproduction steps. If the
private form is temporarily unavailable, create a minimal public issue asking
the maintainer to enable a private reporting channel and omit all sensitive
details.
