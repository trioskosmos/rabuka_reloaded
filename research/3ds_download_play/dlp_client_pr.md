# libctru DLP Client PR (#502)

## Status: OPEN (not merged)

**URL**: https://github.com/devkitPro/libctru/pull/502

## Summary

PR by Pixel-Pop adding DLP client support to libctru. Has a working example.

## Key Comments

### Pixel-Pop (PR author):
> A working example is available here: https://github.com/Pixel-Pop/3ds-examples/tree/master/network/dlpclnt

### ThatHackerDudeFromCyberspace:
> Is there any code for a dlp server?

### TuxSH (libctru maintainer, May 2025):
> Code looks good to me at first glance, but we have a few questions:
> - do you have a server example somewhere?
> - what projects are currently being blocked by this not being merged?

## Fork with the code

MegaPiggy has a fork with the code merged:
https://github.com/MegaPiggy/libctru/commit/c8b503a80081299a9ad1012907b8c6f394bf1df

## What this means

1. **Client-side DLP works** - The PR adds `dlp:CLNT` service wrappers
2. **No server example** - The maintainer specifically asked for a server example
3. **Not in mainline libctru** - Would need to use MegaPiggy's fork or cherry-pick the commits
4. **PR was locked** - "too heated" discussion, locked by devkitPro

## The DLP Server Problem

The DLP server (`dlp:SRVR`) is a **system service** - it's part of the 3DS OS, not something you can call from homebrew directly in the same way. The system's Download Play applet handles the server side.

For homebrew to act as a DLP server, you'd need to either:
1. Use the system's dlp:SRVR service (if accessible from homebrew)
2. Implement the DLP protocol yourself over UDS (complex)
3. Just use UDS directly (simpler)
