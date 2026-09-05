// 把 pnpm 官方 tarball 预置进安装包（src-tauri/resources/pnpm/pnpm-<ver>.tgz）。
//
// 桌面端首次启动需要 pnpm 来安装 dsh 核心；预置后用户机器不再联网下载 pnpm，
// 只有 dsh 本身在启动时安装。版本号与 SHA-256 直接从 Rust 常量文件解析，避免
// 两处维护漂移（Rust 侧在读取随包资源时会再校验一次同一摘要）。
//
// 用法：tsx scripts/fetch-pnpm.ts（已挂在 `pnpm build` 前，tauri build/dev 自动执行）
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join, relative, resolve } from 'node:path'
import process from 'node:process'

const root = resolve(import.meta.dirname, '..')
const constantsRs = join(root, 'src-tauri', 'src', 'config', 'constants.rs')
const outDir = join(root, 'src-tauri', 'resources', 'pnpm')

const source = readFileSync(constantsRs, 'utf8')
const version = source.match(/pub const PNPM_VERSION: &str = "([^"]+)"/)?.[1]
const sha256 = source.match(/pub const PNPM_SHA256: &str = "([0-9a-f]{64})"/)?.[1]
if (!version || !sha256)
  throw new Error(`Cannot parse PNPM_VERSION / PNPM_SHA256 from ${relative(root, constantsRs)}`)

const filename = `pnpm-${version}.tgz`
const target = join(outDir, filename)
const sources = [
  `https://registry.npmjs.org/pnpm/-/${filename}`,
  `https://registry.npmmirror.com/pnpm/-/${filename}`,
]

function digest(buffer: Uint8Array): string {
  return createHash('sha256').update(buffer).digest('hex')
}

// 目录里的一切都会被 tauri 打进安装包，升级版本后残留的旧 tgz 必须清掉
if (existsSync(outDir)) {
  for (const entry of readdirSync(outDir)) {
    if (entry !== filename) {
      rmSync(join(outDir, entry), { recursive: true, force: true })
      console.log(`[fetch-pnpm] removed stale ${relative(root, join(outDir, entry))}`)
    }
  }
}

if (existsSync(target) && digest(readFileSync(target)) === sha256) {
  console.log(`[fetch-pnpm] ${relative(root, target)} already present, skipping`)
  process.exit(0)
}

let buffer: Uint8Array | undefined
let lastError = ''
for (const url of sources) {
  try {
    console.log(`[fetch-pnpm] downloading ${url}`)
    const response = await fetch(url, { signal: AbortSignal.timeout(120_000) })
    if (!response.ok)
      throw new Error(`HTTP ${response.status}`)
    const candidate = new Uint8Array(await response.arrayBuffer())
    const actual = digest(candidate)
    if (actual !== sha256)
      throw new Error(`SHA-256 mismatch: expected ${sha256}, got ${actual}`)
    buffer = candidate
    break
  }
  catch (error) {
    lastError = error instanceof Error ? error.message : String(error)
    console.warn(`[fetch-pnpm] ${url} failed: ${lastError}`)
  }
}

if (!buffer) {
  console.error(`[fetch-pnpm] all sources failed: ${lastError}`)
  process.exit(1)
}

mkdirSync(outDir, { recursive: true })
writeFileSync(target, buffer)
console.log(`[fetch-pnpm] saved ${relative(root, target)} (${buffer.byteLength} bytes)`)
