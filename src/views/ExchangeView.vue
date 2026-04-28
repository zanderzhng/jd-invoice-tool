<template>
  <div class="exchange-view">
    <div class="page-header">
      <div class="header-main">
        <h2>换开发票</h2>
      </div>

      <div class="actions">
        <button class="btn btn-primary" :disabled="store.loading || bulkSubmitting" @click="loadGroups">
          {{ fetchButtonText }}
        </button>
        <div class="exchange-mode-switch" aria-label="换开方式">
          <button class="mode-btn" :class="{ active: !useInvoiceCombos }" @click="useInvoiceCombos = false">
            单开
          </button>
          <button class="mode-btn" :class="{ active: useInvoiceCombos }" @click="useInvoiceCombos = true">
            合开
          </button>
        </div>
        <button class="btn btn-secondary" :disabled="store.loading || bulkSubmitting" @click="showFilters = !showFilters">
          {{ showFilters ? '收起筛选' : '筛选条件' }}
        </button>
      </div>
    </div>

    <div v-if="showFilters" class="filter-panel card">
      <div class="filter-grid">
        <label class="field">
          <span class="field-label">单张发票金额下限（含）</span>
          <input v-model="filters.amountMin" type="number" min="0" step="0.01" class="input" placeholder="不限" />
        </label>

        <label class="field">
          <span class="field-label">单张发票金额上限（含）</span>
          <input v-model="filters.amountMax" type="number" min="0" step="0.01" class="input" placeholder="不限" />
        </label>

        <label v-if="mode === 'smart'" class="field">
          <span class="field-label">组合金额下限（含）</span>
          <input v-model="filters.comboAmountMin" type="number" min="0" step="0.01" class="input" placeholder="不限" />
        </label>

        <label v-if="mode === 'smart'" class="field">
          <span class="field-label">组合金额上限（含）</span>
          <input v-model="filters.comboAmountMax" type="number" min="0" step="0.01" class="input" placeholder="不限" />
        </label>

        <label class="field">
          <span class="field-label">开票日期开始</span>
          <input v-model="filters.dateStart" type="date" class="input" />
        </label>

        <label class="field">
          <span class="field-label">开票日期结束</span>
          <input v-model="filters.dateEnd" type="date" class="input" />
        </label>

        <label class="field">
          <span class="field-label">现有发票抬头</span>
          <select v-model="filters.ivcTitles" class="input multi-select" multiple>
            <option v-for="title in titleOptions" :key="title" :value="title">
              {{ title }}
            </option>
          </select>
        </label>
      </div>

      <div class="filter-actions">
        <div class="filter-buttons">
          <button class="btn btn-secondary" @click="resetFilters">重置筛选</button>
        </div>
      </div>
    </div>

    <div v-if="store.error" class="error-msg">
      {{ store.error }}
    </div>

    <div v-if="unresolvedAmountCount > 0" class="warning-msg">
      当前有 {{ unresolvedAmountCount }} 张订单金额未校准，仍在使用换开页原始金额兜底。请打开控制台查看未匹配的
      `orderId`。
    </div>

    <div v-if="useInvoiceCombos && heldOrders.length > 0" class="hold-panel card">
      <div class="hold-header">
        <div>
          <h3>已 Hold 发票</h3>
          <p>这些发票暂不参与智能凑单，推荐组合已基于剩余发票重新生成。</p>
        </div>
        <button class="btn btn-secondary btn-small" @click="clearHeldOrders">全部取消 Hold</button>
      </div>

      <div class="hold-list">
        <div v-for="order in heldOrders" :key="getMergeOrderId(order)" class="hold-item">
          <div class="hold-main">
            <strong>{{ getMergeOrderSku(order) }}</strong>
            <span>￥{{ getMergeOrderAmount(order).toFixed(2) }}</span>
            <span>{{ formatMergeDateTime(getMergeOrderInvoiceTime(order)) }}</span>
          </div>
          <button class="hold-remove" type="button" @click="toggleHoldOrder(order)">取消 Hold</button>
        </div>
      </div>
    </div>

    <div v-if="!store.hasFetchedMergeGroups && !store.loading" class="empty">
      <p>点击按钮获取可换开的发票分组</p>
      <button class="btn btn-primary" @click="loadGroups">重新获取</button>
    </div>

    <div v-else-if="displayGroups.length === 0 && store.hasFetchedMergeGroups && !store.loading" class="empty">
      <p>{{ emptyMessage }}</p>
    </div>

    <div v-else class="groups">
      <div class="summary-bar">
        <p class="summary">{{ summaryText }}</p>
        <button
          v-if="showBulkExchangeButton"
          class="btn btn-primary bulk-btn"
          :disabled="store.loading || submittingExchange || bulkSubmitting || selectedBulkGroups.length === 0"
          @click="handleBulkExchange"
        >
          {{ bulkSubmitting ? `批量换开中 (${bulkProgressText})` : bulkButtonText }}
        </button>
      </div>

      <InvoiceGroupTable
        :key="mode"
        :groups="displayGroups"
        :secondary-groups="secondaryDisplayGroups"
        :held-order-ids="heldOrderIds"
        :mode="mode"
        @exchange="handleExchange"
        @selection-change="selectedBulkGroups = $event"
        @toggle-hold="toggleHoldOrder"
      />
    </div>

    <LoadingOverlay :visible="store.loading" text="正在获取换开发票数据..." />

    <ExchangeConfirmDialog
      :visible="showConfirmDialog"
      :group="confirmGroup"
      :selected-orders="confirmOrders"
      :titles="store.titles"
      :submitting="submittingExchange"
      @close="showConfirmDialog = false"
      @confirm="handleConfirmExchange"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { submitMergeExchange } from '@/api'
import InvoiceGroupTable from '@/components/InvoiceGroupTable.vue'
import LoadingOverlay from '@/components/LoadingOverlay.vue'
import ExchangeConfirmDialog from '@/components/ExchangeConfirmDialog.vue'
import { useAppStore } from '@/stores/app'
import type { DisplayMergeGroup, ExchangeMode, MergeFilters, MergeOrder } from '@/types'
import {
  buildOriginalDisplayGroups,
  buildSmartMergeGroups,
  DEFAULT_MERGE_FILTERS,
  filterMergeGroups,
  formatMergeDateTime,
  getMergeOrderAmount,
  getMergeOrderId,
  getMergeOrderInvoiceTime,
  getMergeOrderSku,
  getMergeOrderTitle,
  isMergeOrderAmountResolved,
} from '@/utils/mergeOrder'

const store = useAppStore()

const useInvoiceCombos = ref(false)
const mode = computed<ExchangeMode>(() => (useInvoiceCombos.value ? 'smart' : 'original'))
const showFilters = ref(false)
const showConfirmDialog = ref(false)
const submittingExchange = ref(false)
const bulkSubmitting = ref(false)
const bulkProgressText = ref('')
const confirmGroup = ref<DisplayMergeGroup | null>(null)
const confirmOrders = ref<MergeOrder[]>([])
const selectedBulkGroups = ref<DisplayMergeGroup[]>([])
const heldOrderIds = ref<Set<string>>(new Set())
const filters = reactive<MergeFilters>({ ...DEFAULT_MERGE_FILTERS })
const originalModeFilters = ref<MergeFilters>({
  ...DEFAULT_MERGE_FILTERS,
  amountMin: '100.00',
})
const smartModeFilters = ref<MergeFilters>({
  ...DEFAULT_MERGE_FILTERS,
  amountMax: '99.99',
  comboAmountMin: '100.00',
})
const originalFiltersInitialized = ref(false)
const smartFiltersInitialized = ref(false)

onMounted(() => {
  void store.loadTitles()
  if (!store.hasFetchedMergeGroups) {
    store.loadCachedMergeGroups()
  }
})

const allOrders = computed(() => store.mergeGroups.flatMap((group) => group.orders))
const unresolvedAmountCount = computed(() => allOrders.value.filter((order) => !isMergeOrderAmountResolved(order)).length)
const heldOrders = computed(() => {
  const result: MergeOrder[] = []
  const seenIds = new Set<string>()

  allOrders.value.forEach((order) => {
    const orderId = getMergeOrderId(order)
    if (!orderId || seenIds.has(orderId) || !heldOrderIds.value.has(orderId)) {
      return
    }

    seenIds.add(orderId)
    result.push(order)
  })

  return result
})

const fetchButtonText = computed(() => {
  if (store.loading) {
    return '处理中...'
  }
  return '重新获取'
})

const titleOptions = computed(() => {
  const titles = new Set<string>()
  allOrders.value.forEach((order) => {
    const title = getMergeOrderTitle(order)
    if (title) {
      titles.add(title)
    }
  })
  return Array.from(titles)
})

function parseFilterNumber(value: string): number | null {
  if (!value) {
    return null
  }

  const parsed = Number.parseFloat(value)
  return Number.isFinite(parsed) ? parsed : null
}

function filterDisplayGroupsByComboAmount(groups: DisplayMergeGroup[], activeFilters: MergeFilters): DisplayMergeGroup[] {
  if (mode.value !== 'smart') {
    return groups
  }

  const min = parseFilterNumber(activeFilters.comboAmountMin)
  const max = parseFilterNumber(activeFilters.comboAmountMax)

  return groups.filter((group) => {
    if (min !== null && group.total < min) {
      return false
    }
    if (max !== null && group.total > max) {
      return false
    }
    return true
  })
}

function getSmartDefaultFilters(): MergeFilters {
  return {
    ...DEFAULT_MERGE_FILTERS,
    amountMax: '99.99',
    comboAmountMin: '100.00',
    ivcTitles: titleOptions.value.filter((title) => !title.includes('公司')),
  }
}

function getSingleDefaultFilters(): MergeFilters {
  return {
    ...DEFAULT_MERGE_FILTERS,
    amountMin: '100.00',
    dateEnd: '2026-01-31',
    ivcTitles: titleOptions.value.filter((title) => !title.includes('公司')),
  }
}

const filteredGroups = computed(() => filterMergeGroups(store.mergeGroups, filters))
const originalDisplayGroups = computed(() => buildOriginalDisplayGroups(filteredGroups.value))
const smartDisplayGroups = computed(() => buildSmartMergeGroups(filteredGroups.value, heldOrderIds.value))

const displayGroups = computed(() => {
  const groups = mode.value === 'smart' ? smartDisplayGroups.value : originalDisplayGroups.value
  return filterDisplayGroupsByComboAmount(groups, filters)
})

const secondaryDisplayGroups = computed(() => {
  if (mode.value !== 'smart') {
    return []
  }

  const visibleGroups = new Set(displayGroups.value)
  return smartDisplayGroups.value.filter((group) => !visibleGroups.has(group))
})

const defaultTitle = computed(() => store.titles.find((item) => item.isDefault) ?? null)

const showBulkExchangeButton = computed(() => {
  return displayGroups.value.length > 0
})

const selectedBulkOrderCount = computed(() => {
  return selectedBulkGroups.value.reduce((sum, group) => sum + group.orders.length, 0)
})

const bulkButtonText = computed(() => {
  if (useInvoiceCombos.value) {
    return `一键按组合换开 (${selectedBulkGroups.value.length})`
  }

  return `一键单独换开 (${selectedBulkOrderCount.value})`
})

const totalAmount = computed(() => displayGroups.value.reduce((sum, group) => sum + group.total, 0).toFixed(2))
const displaySellerCount = computed(() => {
  const sellers = new Set<string>()
  displayGroups.value.forEach((group) => {
    sellers.add(group.sourceOrgName || group.orgName || '未知开票方')
  })
  return sellers.size
})
const displayOrderCount = computed(() => displayGroups.value.reduce((sum, group) => sum + group.orders.length, 0))

function formatFilterAmount(value: number): string {
  return `￥${value.toFixed(2)}`
}

const filterSummaryText = computed(() => {
  const parts: string[] = []
  const amountMin = parseFilterNumber(filters.amountMin)
  const amountMax = parseFilterNumber(filters.amountMax)
  const comboAmountMin = parseFilterNumber(filters.comboAmountMin)
  const comboAmountMax = parseFilterNumber(filters.comboAmountMax)

  if (amountMin !== null) {
    parts.push(`金额大于等于 ${formatFilterAmount(amountMin)}`)
  }
  if (amountMax !== null) {
    parts.push(`金额小于等于 ${formatFilterAmount(amountMax)}`)
  }
  if (mode.value === 'smart' && comboAmountMin !== null) {
    parts.push(`组合金额大于等于 ${formatFilterAmount(comboAmountMin)}`)
  }
  if (mode.value === 'smart' && comboAmountMax !== null) {
    parts.push(`组合金额小于等于 ${formatFilterAmount(comboAmountMax)}`)
  }
  if (filters.dateStart) {
    parts.push(`开票日期不早于 ${filters.dateStart}`)
  }
  if (filters.dateEnd) {
    parts.push(`开票日期不晚于 ${filters.dateEnd}`)
  }
  if (filters.ivcTitles.length > 0) {
    parts.push(`发票抬头是 ${filters.ivcTitles.join('、')}`)
  }

  return parts.length > 0 ? parts.join('，') : '不限'
})

const summaryText = computed(() => {
  const modeSummary = mode.value === 'smart'
    ? `一共 ${displayGroups.value.length} 个推荐组合`
    : `一共 ${displaySellerCount.value} 个开票方`

  return `当前筛选：${filterSummaryText.value}；${modeSummary}，${displayOrderCount.value} 张发票，可换开金额合计 ￥${totalAmount.value}`
})

const emptyMessage = computed(() => {
  if (store.mergeGroups.length === 0) {
    return '暂无可换开的发票数据'
  }
  if (mode.value === 'smart') {
    return filteredGroups.value.length === 0 ? '当前筛选条件下暂无可换开的发票' : '当前筛选条件下暂无满足组合金额条件的推荐组合'
  }
  return '当前筛选条件下暂无可换开的发票'
})

watch(titleOptions, (nextTitles) => {
  if (nextTitles.length === 0) {
    filters.ivcTitles = filters.ivcTitles.filter((title) => nextTitles.includes(title))
    return
  }

  const shouldApplyOriginalDefaults = !originalFiltersInitialized.value && mode.value === 'original'
  const shouldApplySmartDefaults = !smartFiltersInitialized.value && mode.value === 'smart'

  originalModeFilters.value = originalFiltersInitialized.value
    ? {
        ...originalModeFilters.value,
        ivcTitles: originalModeFilters.value.ivcTitles.filter((title) => nextTitles.includes(title)),
      }
    : getSingleDefaultFilters()

  smartModeFilters.value = smartFiltersInitialized.value
    ? {
        ...smartModeFilters.value,
        ivcTitles: smartModeFilters.value.ivcTitles.filter((title) => nextTitles.includes(title)),
      }
    : getSmartDefaultFilters()

  originalFiltersInitialized.value = true
  smartFiltersInitialized.value = true
  if (shouldApplyOriginalDefaults) {
    Object.assign(filters, {
      ...originalModeFilters.value,
      ivcTitles: [...originalModeFilters.value.ivcTitles],
    })
    return
  }

  if (shouldApplySmartDefaults) {
    Object.assign(filters, {
      ...smartModeFilters.value,
      ivcTitles: [...smartModeFilters.value.ivcTitles],
    })
    return
  }

  filters.ivcTitles = filters.ivcTitles.filter((title) => nextTitles.includes(title))
}, { immediate: true })

watch(allOrders, (orders) => {
  const validOrderIds = new Set(orders.map((order) => getMergeOrderId(order)).filter(Boolean))
  const nextHeldOrderIds = new Set(Array.from(heldOrderIds.value).filter((orderId) => validOrderIds.has(orderId)))
  const changed = nextHeldOrderIds.size !== heldOrderIds.value.size
    || Array.from(nextHeldOrderIds).some((orderId) => !heldOrderIds.value.has(orderId))
  if (changed) {
    heldOrderIds.value = nextHeldOrderIds
  }
})

watch(filters, (nextFilters) => {
  const snapshot: MergeFilters = {
    amountMin: nextFilters.amountMin,
    amountMax: nextFilters.amountMax,
    comboAmountMin: nextFilters.comboAmountMin,
    comboAmountMax: nextFilters.comboAmountMax,
    dateStart: nextFilters.dateStart,
    dateEnd: nextFilters.dateEnd,
    ivcTitles: [...nextFilters.ivcTitles],
  }

  if (mode.value === 'smart') {
    smartModeFilters.value = snapshot
    return
  }

  originalModeFilters.value = snapshot
}, { deep: true })

watch(mode, (nextMode) => {
  selectedBulkGroups.value = []
  const nextFilters = nextMode === 'smart' ? smartModeFilters.value : originalModeFilters.value
  Object.assign(filters, {
    ...nextFilters,
    ivcTitles: [...nextFilters.ivcTitles],
  })
}, { immediate: true })

async function loadGroups() {
  await store.loadMergeGroups()
}

function resetFilters() {
  const nextFilters = mode.value === 'smart' ? getSmartDefaultFilters() : getSingleDefaultFilters()
  Object.assign(filters, nextFilters)
}

function getRandomBulkDelayMs(): number {
  return 10_000 + Math.floor(Math.random() * 10_001)
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}

function toggleHoldOrder(order: MergeOrder) {
  const orderId = getMergeOrderId(order)
  if (!orderId) {
    return
  }

  const nextHeldOrderIds = new Set(heldOrderIds.value)
  if (nextHeldOrderIds.has(orderId)) {
    nextHeldOrderIds.delete(orderId)
  } else {
    nextHeldOrderIds.add(orderId)
  }

  heldOrderIds.value = nextHeldOrderIds
  selectedBulkGroups.value = []
}

function clearHeldOrders() {
  heldOrderIds.value = new Set()
  selectedBulkGroups.value = []
}

function handleExchange(group: DisplayMergeGroup, orders: MergeOrder[]) {
  confirmGroup.value = group
  confirmOrders.value = orders
  showConfirmDialog.value = true
}

function resolveTitle(titleId: string) {
  const title = store.titles.find((item) => item.id === titleId)
  if (!title) {
    throw new Error('未找到发票抬头，请重新选择')
  }

  return title
}

async function submitOrders(
  orders: MergeOrder[],
  titleId: string,
  options: { silent?: boolean } = {},
) {
  const { silent = false } = options
  const title = resolveTitle(titleId)

  if (!silent) {
    submittingExchange.value = true
  }

  try {
    const result = await submitMergeExchange(orders, {
      titleType: title.titleType,
      titleName: title.titleName,
      email: title.email,
      taxNo: title.taxNo,
    })

    store.removeMergedOrders(orders)
    return result
  } finally {
    if (!silent) {
      submittingExchange.value = false
    }
  }
}

async function handleConfirmExchange(titleId: string) {
  if (submittingExchange.value || bulkSubmitting.value) {
    return
  }

  try {
    const result = await submitOrders(confirmOrders.value, titleId)
    showConfirmDialog.value = false
    alert(result.message)
  } catch (error) {
    alert(error instanceof Error ? error.message : String(error))
  }
}

async function handleBulkExchange() {
  if (bulkSubmitting.value || submittingExchange.value) {
    return
  }

  const title = defaultTitle.value
  if (!title) {
    alert('请先在抬头管理中设置默认抬头')
    return
  }

  const selectedGroups = selectedBulkGroups.value
  if (selectedGroups.length === 0) {
    alert(useInvoiceCombos.value ? '请先勾选要批量换开的推荐组合' : '请先勾选要单独换开的发票')
    return
  }

  const submitGroups = useInvoiceCombos.value
    ? selectedGroups.filter((group) => group.kind === 'smart')
    : selectedGroups.flatMap((group) => group.orders.map((order, index) => ({
        ...group,
        displayName: `${group.displayName || group.orgName} - 单开 ${index + 1}`,
        orders: [order],
        total: getMergeOrderAmount(order),
      })))

  if (submitGroups.length === 0) {
    alert(useInvoiceCombos.value ? '请先勾选要批量换开的推荐组合' : '请先勾选要单独换开的发票')
    return
  }

  const confirmed = window.confirm(
    useInvoiceCombos.value
      ? `将使用默认抬头“${title.titleName}”按组合换开选中的 ${submitGroups.length} 个组合，是否继续？`
      : `将使用默认抬头“${title.titleName}”单独换开选中的 ${submitGroups.length} 张发票，是否继续？`,
  )
  if (!confirmed) {
    return
  }

  bulkSubmitting.value = true
  const failures: string[] = []

  try {
    for (let index = 0; index < submitGroups.length; index += 1) {
      const group = submitGroups[index]
      bulkProgressText.value = `${index + 1}/${submitGroups.length}`

      try {
        await submitOrders(group.orders, title.id, { silent: true })
      } catch (error) {
        const reason = error instanceof Error ? error.message : String(error)
        failures.push(`${group.displayName || group.orgName}: ${reason}`)
      }

      if (index < submitGroups.length - 1) {
        const delayMs = getRandomBulkDelayMs()
        bulkProgressText.value = `${index + 1}/${submitGroups.length}，等待 ${Math.ceil(delayMs / 1000)} 秒`
        await sleep(delayMs)
      }
    }

    const successCount = submitGroups.length - failures.length
    if (failures.length === 0) {
      alert(useInvoiceCombos.value
        ? `批量换开完成，共成功提交 ${successCount} 个组合。`
        : `批量换开完成，共成功提交 ${successCount} 张发票。`)
      return
    }

    alert(useInvoiceCombos.value
      ? `批量换开已完成，成功 ${successCount} 个组合，失败 ${failures.length} 个。\n\n${failures.join('\n')}`
      : `批量换开已完成，成功 ${successCount} 张发票，失败 ${failures.length} 张。\n\n${failures.join('\n')}`)
  } finally {
    bulkSubmitting.value = false
    bulkProgressText.value = ''
  }
}
</script>

<style scoped>
.exchange-view {
  display: grid;
  gap: 16px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
}

.header-main {
  display: grid;
  gap: 12px;
}

.page-header h2 {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.exchange-mode-switch {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px;
  border-radius: 999px;
  background: #f4f4f4;
}

.mode-btn {
  border: none;
  background: transparent;
  color: #666;
  padding: 8px 14px;
  border-radius: 999px;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s ease;
}

.mode-btn.active {
  background: #e2231a;
  color: #fff;
  box-shadow: 0 4px 10px rgba(226, 35, 26, 0.2);
}

.actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.filter-panel {
  margin-bottom: 0;
}

.warning-msg {
  padding: 12px 14px;
  border-radius: 10px;
  background: #fff8e6;
  color: #9a6700;
  border: 1px solid #f0d79f;
  font-size: 14px;
}

.hold-panel {
  display: grid;
  gap: 12px;
}

.hold-header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: flex-start;
}

.hold-header h3 {
  margin: 0 0 4px;
  font-size: 15px;
}

.hold-header p {
  margin: 0;
  color: #666;
  font-size: 13px;
}

.hold-list {
  display: grid;
  gap: 8px;
}

.hold-item {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  align-items: center;
  padding: 8px 10px;
  border: 1px solid #f0f0f0;
  border-radius: 6px;
  background: #fff;
}

.hold-main {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  color: #666;
  font-size: 13px;
}

.hold-main strong {
  color: #333;
  max-width: 420px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.hold-remove {
  border: 0;
  background: transparent;
  color: #e2231a;
  cursor: pointer;
  white-space: nowrap;
}

.btn-small {
  padding: 6px 10px;
  font-size: 13px;
  white-space: nowrap;
}

.filter-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
}

.field {
  display: grid;
  gap: 6px;
}

.field-label {
  font-size: 13px;
  color: #666;
}

.multi-select {
  min-height: 112px;
}

.filter-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid #f0f0f0;
}

.filter-tip {
  font-size: 13px;
  color: #666;
}

.filter-buttons {
  display: flex;
  gap: 10px;
}

.empty {
  text-align: center;
  padding: 60px 20px;
  color: #999;
}

.empty p {
  margin-bottom: 16px;
  font-size: 16px;
}

.empty-actions {
  display: flex;
  justify-content: center;
  gap: 10px;
}

.summary-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 16px;
}

.summary {
  font-size: 14px;
  color: #666;
  margin: 0;
  padding: 10px 14px;
  background: #fffbe6;
  border: 1px solid #ffe58f;
  border-radius: 6px;
  flex: 1;
}

.bulk-btn {
  white-space: nowrap;
}

.groups {
  display: grid;
  gap: 12px;
}

@media (max-width: 720px) {
  .page-header {
    flex-direction: column;
    align-items: stretch;
  }

  .summary-bar,
  .actions,
  .hold-header,
  .hold-item,
  .hold-main,
  .filter-actions,
  .empty-actions,
  .filter-buttons {
    flex-direction: column;
  }

  .summary-bar .btn,
  .actions .btn,
  .filter-buttons .btn,
  .filter-actions .btn,
  .empty-actions .btn {
    width: 100%;
  }

  .filter-actions {
    align-items: stretch;
  }
}
</style>
