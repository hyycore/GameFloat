const zlib = require('zlib')
const fs = require('fs')
const path = require('path')

const SS = 4

function crc32(buf) {
  let c = ~0
  for (let i = 0; i < buf.length; i++) {
    c ^= buf[i]
    for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1))
  }
  return ~c >>> 0
}

function chunk(type, data) {
  const len = Buffer.alloc(4)
  len.writeUInt32BE(data.length, 0)
  const t = Buffer.from(type, 'ascii')
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])), 0)
  return Buffer.concat([len, t, data, crc])
}

function encodePng(size, rgba) {
  const raw = Buffer.alloc(size * (1 + size * 4))
  for (let y = 0; y < size; y++) {
    const off = y * (1 + size * 4)
    raw[off] = 0
    rgba.copy(raw, off + 1, y * size * 4, (y + 1) * size * 4)
  }
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(size, 0)
  ihdr.writeUInt32BE(size, 4)
  ihdr[8] = 8
  ihdr[9] = 6
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', ihdr),
    chunk('IDAT', zlib.deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0))
  ])
}

function blend(buf, i, r, g, b, a) {
  if (a <= 0) return
  const sa = a / 255
  const da = buf[i + 3] / 255
  const oa = sa + da * (1 - sa)
  if (oa <= 0) return
  buf[i] = Math.round((r * sa + buf[i] * da * (1 - sa)) / oa)
  buf[i + 1] = Math.round((g * sa + buf[i + 1] * da * (1 - sa)) / oa)
  buf[i + 2] = Math.round((b * sa + buf[i + 2] * da * (1 - sa)) / oa)
  buf[i + 3] = Math.round(oa * 255)
}

function fillRoundedRadii(buf, N, x, y, w, h, radii, color) {
  const cx = x + w / 2
  const cy = y + h / 2
  const hw = w / 2
  const hh = h / 2
  const ix0 = Math.max(0, Math.floor(x) - 1)
  const iy0 = Math.max(0, Math.floor(y) - 1)
  const ix1 = Math.min(N - 1, Math.ceil(x + w) + 1)
  const iy1 = Math.min(N - 1, Math.ceil(y + h) + 1)
  for (let py = iy0; py <= iy1; py++) {
    for (let px = ix0; px <= ix1; px++) {
      const x0 = px + 0.5
      const y0 = py + 0.5
      const left = x0 < cx
      const top = y0 < cy
      const r = top ? (left ? radii[0] : radii[1]) : left ? radii[3] : radii[2]
      const qx = Math.abs(x0 - cx) - (hw - r)
      const qy = Math.abs(y0 - cy) - (hh - r)
      if (Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) - r <= 0) {
        blend(buf, (py * N + px) * 4, color[0], color[1], color[2], color[3])
      }
    }
  }
}

function fillRounded(buf, N, x, y, w, h, r, color) {
  fillRoundedRadii(buf, N, x, y, w, h, [r, r, r, r], color)
}

function fillCircle(buf, N, cx, cy, r, color) {
  const ix0 = Math.max(0, Math.floor(cx - r) - 1)
  const iy0 = Math.max(0, Math.floor(cy - r) - 1)
  const ix1 = Math.min(N - 1, Math.ceil(cx + r) + 1)
  const iy1 = Math.min(N - 1, Math.ceil(cy + r) + 1)
  for (let py = iy0; py <= iy1; py++) {
    for (let px = ix0; px <= ix1; px++) {
      const dx = px + 0.5 - cx
      const dy = py + 0.5 - cy
      if (dx * dx + dy * dy <= r * r) {
        blend(buf, (py * N + px) * 4, color[0], color[1], color[2], color[3])
      }
    }
  }
}

function render(size) {
  const N = size * SS
  const buf = new Uint8Array(N * N * 4)

  const BLUE = [17, 17, 17, 255]
  const WHITE = [255, 255, 255, 255]
  const BORDER = [224, 229, 238, 255]

  const pad = N * 0.035
  const radius = N * 0.235
  fillRounded(buf, N, pad, pad, N - pad * 2, N - pad * 2, radius, BORDER)
  const edge = N * 0.012
  fillRounded(
    buf,
    N,
    pad + edge,
    pad + edge,
    N - (pad + edge) * 2,
    N - (pad + edge) * 2,
    radius - edge,
    WHITE
  )

  const wx = N * 0.22
  const wy = N * 0.26
  const ww = N * 0.56
  const wh = N * 0.48
  const wr = N * 0.1
  const stroke = N * 0.052
  const header = N * 0.15

  fillRounded(buf, N, wx, wy, ww, wh, wr, BLUE)
  fillRoundedRadii(
    buf,
    N,
    wx + stroke,
    wy + header,
    ww - stroke * 2,
    wh - header - stroke,
    [0, 0, Math.max(0, wr - stroke), Math.max(0, wr - stroke)],
    WHITE
  )
  fillCircle(buf, N, wx + ww * 0.2, wy + header / 2, N * 0.031, WHITE)

  const out = Buffer.alloc(size * size * 4)
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      let pr = 0
      let pg = 0
      let pb = 0
      let pa = 0
      for (let dy = 0; dy < SS; dy++) {
        for (let dx = 0; dx < SS; dx++) {
          const i = ((y * SS + dy) * N + (x * SS + dx)) * 4
          const a = buf[i + 3]
          pr += buf[i] * a
          pg += buf[i + 1] * a
          pb += buf[i + 2] * a
          pa += a
        }
      }
      const o = (y * size + x) * 4
      const total = SS * SS
      out[o] = pa > 0 ? Math.round(pr / pa) : 0
      out[o + 1] = pa > 0 ? Math.round(pg / pa) : 0
      out[o + 2] = pa > 0 ? Math.round(pb / pa) : 0
      out[o + 3] = Math.round(pa / total)
    }
  }
  return out
}

function encodeBmp(size, rgba) {
  const header = Buffer.alloc(40)
  header.writeUInt32LE(40, 0)
  header.writeInt32LE(size, 4)
  header.writeInt32LE(size * 2, 8)
  header.writeUInt16LE(1, 12)
  header.writeUInt16LE(32, 14)
  header.writeUInt32LE(0, 16)
  header.writeUInt32LE(size * size * 4, 20)

  const pixels = Buffer.alloc(size * size * 4)
  const maskRow = Math.ceil(size / 32) * 4
  const mask = Buffer.alloc(maskRow * size)

  for (let y = 0; y < size; y++) {
    const srcY = size - 1 - y
    for (let x = 0; x < size; x++) {
      const s = (srcY * size + x) * 4
      const d = (y * size + x) * 4
      pixels[d] = rgba[s + 2]
      pixels[d + 1] = rgba[s + 1]
      pixels[d + 2] = rgba[s]
      pixels[d + 3] = rgba[s + 3]
      if (rgba[s + 3] < 128) {
        mask[y * maskRow + (x >> 3)] |= 0x80 >> (x & 7)
      }
    }
  }

  return Buffer.concat([header, pixels, mask])
}

function toIco(images) {
  const header = Buffer.alloc(6)
  header.writeUInt16LE(0, 0)
  header.writeUInt16LE(1, 2)
  header.writeUInt16LE(images.length, 4)
  const entries = Buffer.alloc(16 * images.length)
  let offset = 6 + 16 * images.length
  const blobs = []
  images.forEach((image, index) => {
    const entry = entries.subarray(index * 16, (index + 1) * 16)
    entry[0] = image.size >= 256 ? 0 : image.size
    entry[1] = image.size >= 256 ? 0 : image.size
    entry.writeUInt16LE(1, 4)
    entry.writeUInt16LE(32, 6)
    entry.writeUInt32LE(image.data.length, 8)
    entry.writeUInt32LE(offset, 12)
    offset += image.data.length
    blobs.push(image.data)
  })
  return Buffer.concat([header, entries, ...blobs])
}

const outDir = path.join(__dirname, '..', 'src-tauri', 'icons')
fs.mkdirSync(outDir, { recursive: true })

const png = (size) => encodePng(size, render(size))

const icoSizes = [16, 24, 32, 48, 64, 128]
const icoImages = icoSizes.map((size) => ({ size, data: encodeBmp(size, render(size)) }))
icoImages.push({ size: 256, data: png(256) })
const ico = toIco(icoImages)

fs.writeFileSync(path.join(outDir, '32x32.png'), png(32))
fs.writeFileSync(path.join(outDir, '128x128.png'), png(128))
fs.writeFileSync(path.join(outDir, '128x128@2x.png'), png(256))
fs.writeFileSync(path.join(outDir, 'icon.png'), png(512))
fs.writeFileSync(path.join(outDir, 'icon.ico'), ico)

console.log('icons generated:', fs.readdirSync(outDir).join(', '))
