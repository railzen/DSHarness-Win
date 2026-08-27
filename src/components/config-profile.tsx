import { Plus } from '@gravity-ui/icons'
import { Button, Checkbox, Chip, Description, Input, Label } from '@heroui/react'
import { useOverlay } from '@overlastic/react'
import { useState } from 'react'

import { useTranslation } from 'react-i18next'
import { If } from 'react-if-lite'
import { store } from '@/store'
import { toast } from '@/utils/toast'
import { useDshProfiles } from '../hooks/use-dsh-profiles'
import { Ellipsis } from './ellipsis'
import { Item } from './item'
import { Modal } from './modal'
import { PanelHeader } from './panel-header'
import { PanelState } from './panel-state'

export function ConfigProfile() {
  /**
   * 「档案」面板：展示 & 切换 dsh 配置档案，支持新建/删除。
   *
   * 数据来自 `useDshProfiles`（`get_profiles` 查询 + `setting_updated` 事件刷新）：
   * 档案 = `$DSH_HOME/profiles/<id>` 目录，与官方 dsh CLI 的 profile 语义一致；
   * 桌面端把「当前档案」持久化在 store（`active_profile`），服务启动与配置隔离
   * 都以它为准。切换档案需要重启服务才生效（toast 内提供重启入口）。
   */
  const { profiles, loading, error, createProfile, activateProfile, removeProfile, busy } = useDshProfiles()

  const [dialogHolder, openDialog] = useOverlay(Modal, { type: 'holder' })

  const { t } = useTranslation()
  const [creating, setCreating] = useState(false)
  const [name, setName] = useState('')

  async function activate(id: string) {
    const target = profiles.find(p => p.id === id)
    if (!target || target.active || busy)
      return
    try {
      await openDialog({
        status: 'warning',
        title: t('profiles.activate_confirm_title'),
        description: (
          <p>
            {t('profiles.activate_confirm_desc', { name: target.name })}
          </p>
        ),
      })
    }
    catch {
      return
    }
    try {
      await activateProfile(id)
      const key = toast(t('profiles.activate_toast', { name: target.name }), {
        variant: 'accent',
        description: t('profiles.activate_restart_hint'),
        timeout: 10_000,
        actionProps: {
          children: t('app.restart'),
          onPress: () => {
            store.harness.restart()
            toast.close(key)
          },
        },
      })
    }
    catch (err) {
      console.error('[ConfigProfile] activate failed:', err)
      toast(t('profiles.activate_failed', { name: target.name }), {})
    }
  }

  function startCreate() {
    setCreating(true)
    setName('')
  }

  function cancelCreate() {
    setCreating(false)
    setName('')
  }

  async function commitCreate() {
    const trimmed = name.trim()
    if (!trimmed)
      return
    try {
      // 创建成功后列表已刷新出新档案，UI 本身就有变化，不再弹成功 toast
      await createProfile(trimmed)
      setCreating(false)
      setName('')
    }
    catch (err) {
      console.error('[ConfigProfile] create failed:', err)
      toast(t('profiles.create_failed'), {})
    }
  }

  async function remove(id: string) {
    const target = profiles.find(p => p.id === id)
    if (!target || busy)
      return
    try {
      await openDialog({
        title: t('profiles.remove_confirm_title'),
        status: 'danger',
        description: (
          <p>
            {t('profiles.remove_confirm_desc', { name: target.name })}
          </p>
        ),
        confirmText: t('profiles.remove_confirm'),
      })
    }
    catch {
      return
    }
    try {
      // 删除成功后列表已移除该档案，UI 本身就有变化，不再弹成功 toast
      await removeProfile(id)
    }
    catch (err) {
      console.error('[ConfigProfile] remove failed:', err)
      toast(t('profiles.remove_failed'), {})
    }
  }

  return (
    <div className="space-y-3">
      <PanelHeader title={t('profiles.title')} description={t('profiles.tooltip')} />

      {/* 加载 / 失败 / 列表 */}
      <PanelState loading={loading} error={error}>
        <div className="flex flex-col gap-4">
          {profiles.map(profile => (
            <Item
              key={profile.id}
              onClick={() => activate(profile.id)}
              left={(
                <>
                  <Label className="min-w-0 truncate text-sm font-medium text-ink">
                    {profile.name}
                  </Label>
                  <If cond={profile.default}>
                    <Description className="min-w-0 text-xs text-muted">
                      <Ellipsis>{t('profiles.default_desc')}</Ellipsis>
                    </Description>
                  </If>
                </>
              )}
              right={(
                <>
                  <Checkbox
                    isSelected={profile.active}
                    isDisabled={busy}
                    aria-label={profile.name}
                    className="shrink-0"
                  >
                    <Checkbox.Content>
                      <Checkbox.Control>
                        <Checkbox.Indicator />
                      </Checkbox.Control>
                    </Checkbox.Content>
                  </Checkbox>
                  <If cond={!profile.default}>
                    <Chip
                      className="rounded-md"
                      variant="primary"
                      color="danger"
                      size="sm"
                      onClick={(event) => {
                        event.stopPropagation()
                        remove(profile.id)
                      }}
                    >
                      {t('profiles.remove')}
                    </Chip>
                  </If>
                </>
              )}
            />
          ))}
          {/* 新建档案：内联输入 or 触发入口 */}
          <If
            cond={!creating}
            else={(
              <div className="flex items-center gap-2 px-1">
                <Input
                  autoFocus
                  variant="secondary"
                  className="h-8 flex-1 rounded-md"
                  placeholder={t('profiles.name_placeholder')}
                  value={name}
                  onChange={e => setName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter')
                      commitCreate()
                  }}
                />
                <Button size="sm" variant="tertiary" className="h-8 rounded-md" onPress={cancelCreate}>
                  {t('profiles.create_cancel')}
                </Button>
                <Button
                  size="sm"
                  variant="primary"
                  className="h-8 rounded-md"
                  isDisabled={!name.trim() || busy}
                  onPress={commitCreate}
                >
                  {t('profiles.create_confirm')}
                </Button>
              </div>
            )}
          >
            <Button
              onClick={startCreate}
              variant="tertiary"
              className="flex w-full rounded-md"
              isDisabled={busy}
            >
              <Plus className="size-3.5" />
              <span>{t('profiles.new_profile')}</span>
            </Button>
          </If>
        </div>
      </PanelState>
      {dialogHolder}
    </div>
  )
}
