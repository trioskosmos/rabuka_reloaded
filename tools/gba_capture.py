"""Minimal GDB Remote Serial Protocol client for mGBA.

mGBA 0.10.5's built-in GDB server times out against modern GDB's capability
handshake, so we speak the bare minimum protocol ourselves: connect, continue,
halt, read memory, detach. That is enough to snapshot VRAM/OAM/palettes at an
exact moment and hand them to the renderer.

Usage:
    python gba_capture.py <rom> <out.png> [--seconds N] [--label TEXT]

Starts mGBA itself with --gdb, plays for N seconds, halts the CPU, dumps the
display state and renders a PNG.
"""

import os
import socket
import struct
import subprocess
import sys
import time

MGBA = r"C:\Program Files\mGBA\mGBA.exe"
GDB_PORT = 2345

W, H = 240, 160

REG_DISPCNT = 0x04000000
REG_BGCNT = 0x04000008
PALETTE_BG = 0x05000000
PALETTE_OBJ = 0x05000200
VRAM = 0x06000000
VRAM_SIZE = 0x00018000
OAM = 0x07000000


def checksum(payload: str) -> str:
    return "%02x" % (sum(ord(c) for c in payload) & 0xFF)


class Rsp:
    """Just enough RSP to drive mGBA."""

    def __init__(self, host="127.0.0.1", port=GDB_PORT, timeout=30):
        self.sock = socket.create_connection((host, port), timeout=timeout)
        self.sock.settimeout(timeout)
        self.buf = b""

    def _send(self, payload: str) -> None:
        # Old GDB protocol: a '$'-framed packet with a #xx checksum.
        self.sock.sendall(("$" + payload + "#" + checksum(payload)).encode())

    def _read_byte(self) -> int:
        while not self.buf:
            self.buf = self.sock.recv(4096)
        b, self.buf = self.buf[0], self.buf[1:]
        return b

    def _recv_packet(self) -> str:
        while True:
            b = self._read_byte()
            if b == ord("$"):
                break
            if b == ord("+"):
                continue
        out = b""
        while True:
            b = self._read_byte()
            if b == ord("#"):
                break
            out += bytes([b])
        self._read_byte()  # checksum byte
        self.sock.sendall(b"+")
        return out.decode(errors="replace")

    def cmd(self, payload: str) -> str:
        self._send(payload)
        return self._recv_packet()

    def continue_(self) -> None:
        # No reply expected; '+' ack then silence.
        self._send("c")
        deadline = time.time() + 5
        self.sock.settimeout(0.2)
        try:
            while time.time() < deadline:
                self.sock.recv(4096)
        except (socket.timeout, TimeoutError):
            pass
        self.sock.settimeout(30)

    def halt(self) -> str:
        """Break a running target via an interrupt byte."""
        self.sock.sendall(b"\x03")
        self.sock.settimeout(10)
        try:
            return self._recv_packet()
        except (socket.timeout, TimeoutError):
            return ""
        finally:
            self.sock.settimeout(30)

    def read_mem(self, addr: int, length: int) -> bytes:
        out = bytearray()
        while len(out) < length:
            chunk = min(1024, length - len(out))
            resp = self.cmd("m%x,%x" % (addr + len(out), chunk))
            # mGBA 0.10.5 omits the conventional 'm' prefix on memory replies,
            # and sends 'E<nn>' for unsupported regions. Accept either.
            if resp.startswith("E"):
                raise RuntimeError("read %08x+%d unsupported: %r" % (addr, chunk, resp))
            payload = resp[1:] if resp.startswith("m") else resp
            payload = "".join(c for c in payload if c in "0123456789abcdefABCDEF")
            if not payload:
                raise RuntimeError("empty read at %08x" % (addr + len(out)))
            out += bytes.fromhex(payload)
        return bytes(out[:length])

    def regs(self) -> dict:
        g = self.cmd("g")
        body = "".join(c for c in g if c in "0123456789abcdefABCDEF")
        raw = bytes.fromhex(body)
        out = {}
        for i in range(min(16, len(raw) // 4)):
            out["r%d" % i] = struct.unpack_from("<I", raw, i * 4)[0]
        # mGBA sends r0-r15 then cpsr; sp is r13 and lr is r14 in this layout.
        if "r13" in out:
            out["sp"] = out["r13"]
        if "r14" in out:
            out["lr"] = out["r14"]
        out["pc"] = out.get("r15", 0)
        return out

    def close(self) -> None:
        try:
            self._send("D")
        except OSError:
            pass
        self.sock.close()


def rgb15(word: int):
    r = (word & 0x1F) << 3
    g = ((word >> 5) & 0x1F) << 3
    b = ((word >> 10) & 0x1F) << 3
    return (min(r + (r >> 2), 255), min(g + (g >> 2), 255), min(b + (b >> 2), 255))


# GBA BG sizes, indexed by BGCNT bits 14-15.
BG_TILES = [
    (32, 32), (64, 32), (32, 64), (64, 64),
]


def render(dispcnt, bgcnt, pal_bg, pal_obj, vram, oam, px, py):
    """Software-composite the tiled BG layers plus OBJ into an RGB image."""
    img = [[(0, 0, 0)] * W for _ in range(H)]

    obj_1d = (dispcnt >> 6) & 1
    bg_on = (dispcnt >> 8) & 0xF
    obj_on = (dispcnt >> 12) & 1

    # Draw backgrounds back-to-front: priority 3 first, priority 0 last.
    bgs = []
    for n in range(4):
        if not (bg_on >> n) & 1:
            continue
        bgs.append(((bgcnt[n] & 3), n, bgcnt[n]))
    bgs.sort(reverse=True)

    for prio, n, cnt in bgs:
        charbase = ((cnt >> 2) & 3) << 14
        eight_bpp = (cnt >> 7) & 1
        screenbase = ((cnt >> 8) & 0x1F) << 11
        mw, mh = BG_TILES[(cnt >> 14) & 3]

        for y in range(H):
            ty = ((y + py) >> 3) % mh
            for x in range(W):
                tx = ((x + px) >> 3) % mw
                off = screenbase + (ty * mw + tx) * 2
                if off + 1 >= len(vram):
                    continue
                entry = vram[off] | (vram[off + 1] << 8)
                tile = entry & 0x3FF
                hflip = (entry >> 10) & 1
                vflip = (entry >> 11) & 1
                bank = (entry >> 12) & 0xF
                ix, iy = (x >> 3) & 7, (y >> 3) & 7
                if hflip:
                    ix = 7 - ix
                if vflip:
                    iy = 7 - iy
                if eight_bpp:
                    addr = charbase + tile * 64 + iy * 8 + ix
                    if addr >= len(vram):
                        continue
                    ci = vram[addr]
                    if ci == 0:
                        continue
                    word = pal_obj_placeholder = 0
                    # 8bpp BG palette occupies the low half of BG palette RAM.
                    a = bank * 256 + ci * 2
                    if a + 1 < len(pal_bg):
                        img[y][x] = rgb15(pal_bg[a] | (pal_bg[a + 1] << 8))
                else:
                    addr = charbase + tile * 32 + iy * 4 + (ix >> 1)
                    if addr >= len(vram):
                        continue
                    byte = vram[addr]
                    ci = (byte >> 4) if (ix & 1) else (byte & 0xF)
                    if ci == 0:
                        continue
                    a = bank * 32 + ci * 2
                    if a + 1 < len(pal_bg):
                        img[y][x] = rgb15(pal_bg[a] | (pal_bg[a + 1] << 8))

    # Objects: OAM index 0 is highest priority, so draw 127 -> 0.
    sizes = [(8, 8), (16, 8), (32, 8), (64, 8),
             (8, 16), (16, 16), (32, 16), (64, 16),
             (8, 32), (16, 32), (32, 32), (64, 32),
             (8, 64), (16, 64), (32, 64), (64, 64)]
    if obj_on:
        for i in range(127, -1, -1):
            a0, a1, a2 = oam[i * 4] | (oam[i * 4 + 1] << 8), \
                         oam[i * 4 + 2] | (oam[i * 4 + 3] << 8), \
                         oam[i * 4 + 4] | (oam[i * 4 + 5] << 8)
            if a0 & (1 << 15) == 0:
                continue  # disabled
            if a0 & (1 << 14):
                continue  # affine
            eight = (a0 >> 15) & 1
            if not eight and (a0 & (1 << 14)):
                continue
            y = (a0 & 0xFF)
            if y >= 160:
                y -= 256
            x = a1 & 0x1FF
            if x >= 240:
                x -= 512
            sw, sh = sizes[(a1 >> 12) & 0xF]
            hflip = (a1 >> 11) & 1
            vflip = (a1 >> 9) & 1
            tile = a2 & 0x3FF
            bank = (a2 >> 12) & 0xF
            prio = (a2 >> 10) & 3
            if prio != 1:
                # This port puts card art on P1; drawing P2 here would hide it.
                continue
            tiles_w = sw // 8
            for sy in range(sh):
                for sx in range(sw):
                    dx, dy = x + sx, y + sy
                    if not (0 <= dx < W and 0 <= dy < H):
                        continue
                    tx, ty = sx // 8, sy // 8
                    ix, iy = sx % 8, sy % 8
                    if hflip:
                        ix = 7 - ix
                    if vflip:
                        iy = 7 - iy
                    slot = ty * tiles_w + tx
                    if eight:
                        addr = tile * 64 + slot * 64 + iy * 8 + ix
                        if addr >= len(vram):
                            continue
                        ci = vram[addr]
                        if ci == 0:
                            continue
                        pa = bank * 256 + ci * 2
                        if pa + 1 < len(pal_obj):
                            img[dy][dx] = rgb15(pal_obj[pa] | (pal_obj[pa + 1] << 8))
                    else:
                        addr = tile * 32 + slot * 32 + iy * 4 + (ix >> 1)
                        if addr >= len(vram):
                            continue
                        byte = vram[addr]
                        ci = (byte >> 4) if (ix & 1) else (byte & 0xF)
                        if ci == 0:
                            continue
                        pa = bank * 32 + ci * 2
                        if pa + 1 < len(pal_obj):
                            img[dy][dx] = rgb15(pal_obj[pa] | (pal_obj[pa + 1] << 8))
    return img


def main():
    rom = sys.argv[1]
    out = sys.argv[2]
    seconds = 6.0
    label = ""
    args = sys.argv[3:]
    for i, a in enumerate(args):
        if a == "--seconds":
            seconds = float(args[i + 1])
        if a == "--label":
            label = args[i + 1]

    proc = subprocess.Popen(
        [MGBA, "--gdb", rom],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )
    try:
        time.sleep(2.5)
        rsp = Rsp()
        rsp.cmd("?")
        rsp.continue_()
        time.sleep(seconds)
        rsp.halt()
        try:
            r = rsp.regs()
            print("pc=%08x sp=%08x lr=%08x" % (r["pc"], r.get("sp", 0), r.get("lr", 0)))
        except Exception as exc:  # registers are a nicety, not required
            print("regs unavailable:", exc)
        dispcnt = int.from_bytes(rsp.read_mem(REG_DISPCNT, 4), "little")
        bgcnt = [int.from_bytes(rsp.read_mem(REG_BGCNT + 2 * n, 2), "little") for n in range(4)]
        pal_bg = rsp.read_mem(PALETTE_BG, 0x200)
        pal_obj = rsp.read_mem(PALETTE_OBJ, 0x200)
        vram = rsp.read_mem(VRAM, VRAM_SIZE)
        oam = rsp.read_mem(OAM, 0x400)
        rsp.close()

        img = render(dispcnt, bgcnt, pal_bg, pal_obj, vram, oam, 0, 0)
        from PIL import Image
        out_img = Image.new("RGB", (W, H))
        out_img.putdata([img[y][x] for y in range(H) for x in range(W)])
        out_img.save(out)
        print("dispcnt=%08x bg_on=%x obj_on=%x mode=%d" % (
            dispcnt, (dispcnt >> 8) & 0xF, (dispcnt >> 12) & 1, dispcnt & 3))
        print("saved", out, ("label=" + label) if label else "")
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()


if __name__ == "__main__":
    main()