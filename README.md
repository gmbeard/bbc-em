BBC-em
===
A BBC Micro emulator written in Rust. Currently only works on Windows due to
a platform specific signal handler in the debugger.

There's still a lot of work to do...

- [x] 6502
- [x] 6845CRTC
- [x] Debugger
- [x] Logging
- [x] Display
- [x] Simple interrupt mechanism
- [ ] Keyboard input
- [ ] Timers / VIA
- [ ] Tape / DFS
- [ ] Proper timing tweaks
- [ ] Multi platform
- [ ] ...

Debugger
---
The debugger can be started using the `--debug` command line switch. I don't
recommend running the debugger using Cargo as `CTRL-C` won't be handled
properly. The debugger commands implemented so far are as follows...

- `next [N]` / `n [N]`: steps over `N` instructions (defaults to `1`), printing 
  each one .
- `page N`: prints the 256 bytes of page `N`. `N` should be in hex (`ff`)
  format.
- `c` / `continue`: continues executing (without printing instructions).
  Pressing `CTRL-C` breaks at the nearest instruction.
- `break [ADDRESS]`: Sets a breakpoint at `ADDRESS`.
- `quit`: Quits the debugger.

