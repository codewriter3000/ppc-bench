# PPC-Bench

A PowerPC (Gekko/Broadway) assembly test bench with deep debugging, in the spirit of MARS for MIPS.
Built with Tauri v2 + SolidJS, organized as a microkernel monorepo.

PPC interpreter semantics are ported from the [Dolphin emulator](https://github.com/dolphin-emu/dolphin) (GPL-2.0+).

## Workspace layout

- `apps/desktop` — Tauri shell (Rust PPC engine + SolidJS frontend)
- `packages/kernel` — typed event bus + contracts
- `packages/ui` — shared SolidJS components, panels, design tokens

## Develop

```
npm install
npm run dev:desktop
```

## License

The Rust PPC engine code derived from Dolphin is GPL-2.0+. The rest of the codebase follows the
same license to remain compatible.
