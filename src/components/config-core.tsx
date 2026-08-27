import type { HarnessCore } from '../hooks/use-dsh-cores'
import { ArrowRotateRight, CircleArrowDown as DownloadIcon, FolderOpen } from '@gravity-ui/icons'
import { Button, Chip, Description, Label, Spinner } from '@heroui/react'
import { useOverlay } from '@overlastic/react'
import { invoke } from '@tauri-apps/api/core'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { If } from 'react-if-lite'
import { store } from '@/store'
import { toast } from '@/utils/toast'
import { useDshCores } from '../hooks/use-dsh-cores'
import { DownloadCoreDialog } from './download-core-dialog'
import { Empty } from './empty'
import { Item } from './item'
import { Modal } from './modal'
import { PanelHeader } from './panel-header'
import { PanelState } from './panel-state'

/**
 * 「核心」面板：管理 Harness 引擎来源与多版本。
 *
 * - 列表来自 `useDshCores`（`get_cores` 查询 + `setting_updated` 事件刷新）：
 *   `local` = 用户通过 CLI 全局安装的本地核心（存在时优先使用，需求 3）；
 *   `app-<tag>` = DeepSeek 官方 Release 的各发布版本（GitHub tags 拉取失败时
 *   降级为磁盘扫描，仅显示已下载版本）。
 * - 切换核心：持久化后**自动重启**服务（需求 5），重启走 harness store 的
 *   restart 流程（停止 → 重新启动 → 健康检查）。
 * - 下载版本：拉指定 tag 的发布资产到历史槽位（不激活），随后可切换；
 *   卸载仅允许非激活的已下载版本。
 * - 本地核心更新：通过用户包管理器 CLI（npm install -g @latest / pnpm add -g @latest）。
 * - 每行展示核心入口（cli path，超长省略号 + 限制宽度）。
 */
export function ConfigCore() {
  const [dialogHolder, openDialog] = useOverlay(Modal, { type: 'holder' })
  const [downloadDialogHolder, openDownloadDialog] = useOverlay(DownloadCoreDialog, { type: 'holder' })

  const { t } = useTranslation()
  const { cores, loading, error, setActiveCore, updateLocalCore, downloadCore, uninstallGlobalCore, busy } = useDshCores()

  /** 行内操作进行中的核心 id（该行的下载/卸载按钮显示 Spinner 并禁用重复点击） */
  const [busyId, setBusyId] = useState<string | null>(null)

  // 本地核心未检测到时不渲染 local 行（保留 local_missing_hint 提示）
  const localCore = cores.find(c => c.source === 'local')

  // 本地核心是否有新版可更新：仅当存在更新的预打包发布时才显示「更新本地核心」。
  // 版本行按 tags 最新在前，取第一个 app 版本作为"当前最新可用版本"（本地版本
  // 已是最新时不再展示更新入口，避免"已最新仍提示更新"）。
  const localVersion = localCore?.version ?? ''
  const latestVersion = cores.find(c => c.source === 'app')?.version ?? ''
  const hasLocalUpdate = !!(localCore?.present && localVersion && latestVersion && compareVersions(localVersion, latestVersion) < 0)
  // 官方 tags 全部展示，用户可以随时选择任意版本全局安装。
  const rowsToShow = cores.filter(core => {
    if (core.source === 'local' && !core.present)
      return false
    // 全局当前版本已是最新版时，隐藏同版本的官方重复行。
    if (core.source === 'app' && localCore?.present && core.version === localVersion)
      return false
    return true
  })

  /** 包裹行内操作：全局单例守卫 + 该行 busy 标记 */
  async function runBusy(id: string, action: () => Promise<unknown>) {
    if (busy)
      return
    setBusyId(id)
    try {
      await action()
    }
    finally {
      setBusyId(null)
    }
  }

  async function onActivate(core: HarnessCore) {
    if (core.active || busy || !core.present)
      return
    try {
      await openDialog({
        status: 'warning',
        title: t('core.switch_confirm_title'),
        description: (
          <p>
            {t('core.switch_confirm_desc', { version: displayVersion(core) })}
          </p>
        ),
      })
    }
    catch {
      return
    }
    try {
      await runBusy(core.id, () => setActiveCore(core.id))
      const key = toast(t('core.activate_toast', { version: displayVersion(core) }), {
        variant: 'accent',
        description: t('core.switch_restart_hint'),
        timeout: 10_000,
      })
      // 需求 5：切换核心后自动重启服务；重启结束后收起提示 toast。
      // 重启失败已由应用错误态呈现，这里静默吞掉以免重复弹错。
      void store.harness.restart()
        .then(() => toast.close(key))
        .catch(() => {})
    }
    catch (err) {
      console.error('[ConfigCore] switch failed:', err)
      toast(t('core.switch_failed'), {})
    }
  }

  async function onDownload(core: HarnessCore) {
    if (busy)
      return
    // 下载过程在对话框内展示进度 + 日志（复用 install-progress 事件流）；
    // 对话框 confirm（下载成功）或 cancel（失败后点关闭）都会结束本次等待。
    try {
      await openDownloadDialog({
        tag: core.tag,
        version: displayVersion(core),
        runDownload: tag => downloadCore(tag),
      })
      await store.harness.start()
      toast(t('core.downloaded_toast', { version: displayVersion(core) }), {})
    }
    catch (err) {
      console.error('[ConfigCore] download failed:', err)
      // 失败详情已在下载对话框内展示（含日志），此处不再重复 toast
    }
  }

  /** 打开核心所在目录（文件夹图标） */
  async function openCoreDir(core: HarnessCore) {
    if (!core.dir || busy)
      return
    try {
      await invoke('open_dir', { path: core.dir })
    }
    catch (err) {
      console.error('[ConfigCore] open dir failed:', err)
      toast(t('core.open_dir_failed'), {})
    }
  }

  async function onUpdateLocal() {
    if (busy)
      return
    setBusyId('local')
    try {
      const version = await updateLocalCore()
      await store.harness.start()
      toast(t('core.updated_toast', { version: version || '—' }), {})
    }
    catch (err) {
      console.error('[ConfigCore] update local core failed:', err)
      toast(t('core.update_failed'), {})
    }
    finally {
      setBusyId(null)
    }
  }

  async function onUninstallGlobal() {
    if (busy || !localCore?.present)
      return
    try {
      await openDialog({ status: 'danger', title: t('core.remove_confirm_title'), description: <p>{t('core.remove_confirm_desc', { version: displayVersion(localCore) })}</p>, confirmText: t('core.uninstall') })
      if (localCore.active)
        await store.harness.shutdown()
      await uninstallGlobalCore()
      toast(t('core.uninstalled_toast', { version: displayVersion(localCore) }), {})
    }
    catch (err) {
      console.error('[ConfigCore] global uninstall failed:', err)
      toast(t('core.remove_failed'), {})
    }
  }

  return (
    <div className="space-y-3">
      <PanelHeader title={t('core.title')} description={t('core.tooltip')} />

      {/* 加载 / 失败 / 列表 */}
      <PanelState loading={loading} error={error}>
        <div className="flex flex-col gap-4">
          {rowsToShow.map(core => (
            <Item
              key={core.id}
              onClick={core.source === 'local' && core.present && !core.active ? () => onActivate(core) : undefined}
              left={(
                <>
                  <Label className="min-w-0 truncate font-mono text-sm font-medium text-ink">
                    {displayVersion(core)}
                  </Label>
                  <If cond={core.source === 'local'}>
                    <Chip size="sm" variant="soft" color="accent" className="shrink-0 font-medium">
                      {t('core.local')}
                    </Chip>
                  </If>
                  <If cond={core.version === latestVersion && (core.source === 'app' || core.source === 'local')}>
                    <Chip size="sm" variant="soft" color="default" className="shrink-0 font-medium">
                      {t('core.latest')}
                    </Chip>
                  </If>
                  {/* 已下载：Chip 右侧的文件夹图标，点击打开所在目录 */}
                  <If cond={core.source === 'local' && core.present}>
                    <Button
                      size="sm"
                      variant="tertiary"
                      className="h-6 w-6 shrink-0 rounded-md p-0"
                      isDisabled={busy}
                      aria-label={t('core.open_dir')}
                      onClick={(event) => {
                        event.stopPropagation()
                        openCoreDir(core)
                      }}
                    >
                      <FolderOpen className="size-3.5" />
                    </Button>
                  </If>
                  <If cond={!core.present}>
                    <Description className="min-w-0 text-xs text-muted">
                      {t('core.not_downloaded')}
                    </Description>
                  </If>
                </>
              )}
              right={(
                <>
                  {/* 已下载：切换（选中当前使用版本） */}
                  <If cond={core.source === 'local' && core.present}>
                    <Button size="sm" variant="danger" className="h-7 rounded-md text-xs" isDisabled={busy} onClick={(event) => { event.stopPropagation(); void onUninstallGlobal() }}>
                      {t('core.uninstall')}
                    </Button>
                  </If>
                  {/* 未下载（app 版本）：下载入口（进度与日志在下载对话框内展示） */}
                  <If cond={core.source === 'app'}>
                    <Button
                      size="sm"
                      variant="tertiary"
                      className="h-7 rounded-md text-xs"
                      isDisabled={busy}
                      onClick={(event) => {
                        event.stopPropagation()
                        onDownload(core)
                      }}
                    >
                      <DownloadIcon className="size-3.5" />
                      {t('core.download')}
                    </Button>
                  </If>
                  {/* 已下载且非激活（app 版本）：卸载入口 */}
                  {/* 本地核心：已是最新时不显示；有新版时提供更新入口（与预打包行同栏，统一布局） */}
                  <If cond={core.source === 'local' && core.present && hasLocalUpdate}>
                    <Button
                      size="sm"
                      variant="tertiary"
                      className="h-7 rounded-md text-xs"
                      isDisabled={busy}
                      onClick={(event) => {
                        event.stopPropagation()
                        onUpdateLocal()
                      }}
                    >
                      <If cond={busyId === core.id && busy} then={<Spinner size="sm" color="current" />} else={<ArrowRotateRight className="size-3.5" />} />
                      {t('core.update_local')}
                    </Button>
                  </If>
                </>
              )}
            />
          ))}
          {/* 本地核心提示：未检测到时说明如何安装 */}
          <If cond={!localCore?.present}>
            <Empty>{t('core.local_missing_hint')}</Empty>
          </If>
        </div>
      </PanelState>

      {dialogHolder}
      {downloadDialogHolder}
    </div>
  )
}

/** 版本展示：优先版本号，缺失回落来源 id */
function displayVersion(version: HarnessCore): string {
  return version.version || (version.source === 'local' ? 'local' : 'app')
}

/**
 * 简化 semver 比较（不引入额外依赖），可处理 `0.1.1-rc.2` / `0.1.1` 格式。
 * 返回值：负数 a < b，0 相等，正数 a > b。
 */
function compareVersions(a: string, b: string): number {
  const parse = (v: string) => {
    const [core, pre = ''] = v.split('-', 2)
    const nums = core.split('.').map(n => parseInt(n, 10) || 0)
    return { nums, pre }
  }
  const pa = parse(a)
  const pb = parse(b)
  for (let i = 0; i < 3; i++) {
    const x = pa.nums[i] ?? 0
    const y = pb.nums[i] ?? 0
    if (x !== y)
      return x < y ? -1 : 1
  }
  // 无预发布号 > 有预发布号
  if (!pa.pre && !pb.pre)
    return 0
  if (!pa.pre)
    return 1
  if (!pb.pre)
    return -1
  // 预发布号按点分段比较：数字按数值、非数字按字典序
  const paParts = pa.pre.split('.').map(p => (Number.isNaN(Number(p)) ? p : Number(p)))
  const pbParts = pb.pre.split('.').map(p => (Number.isNaN(Number(p)) ? p : Number(p)))
  const len = Math.max(paParts.length, pbParts.length)
  for (let i = 0; i < len; i++) {
    const x = paParts[i]
    const y = pbParts[i]
    if (x === undefined)
      return -1
    if (y === undefined)
      return 1
    if (x === y)
      continue
    if (typeof x === 'number' && typeof y === 'number')
      return x < y ? -1 : 1
    if (typeof x === 'number')
      return -1
    if (typeof y === 'number')
      return 1
    return x < y ? -1 : 1
  }
  return 0
}
