import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  getCookie,
  saveCookie,
  deleteCookie,
  fetchInvoices,
  fetchBatchOrders,
  checkMerge,
  getTitles,
  saveTitles,
  getUserAgent,
  saveUserAgent,
  resetUserAgent,
} from '@/api'
import type { BatchOrderSeed, InvoiceItem, MergeGroup, MergeOrder } from '@/types'
import { getMergeOrderAmount, isMergeOrderAmountResolved } from '@/utils/mergeOrder'
import type { InvoiceTitle } from '@/types/title'

export const useAppStore = defineStore('app', () => {
  const MERGE_GROUP_CACHE_KEY = 'jd-invoice-tool:merge-groups'

  const isLoggedIn = ref(false)
  const cookie = ref<string | null>(null)
  const invoices = ref<InvoiceItem[]>([])
  const mergeGroups = ref<MergeGroup[]>([])
  const hasFetchedMergeGroups = ref(false)
  const hasLoadedMergeGroupCache = ref(false)
  const loading = ref(false)
  const loadingMore = ref(false)
  const error = ref<string | null>(null)
  const invoicePage = ref(1)
  const invoiceHasMore = ref(true)
  const invoiceSeen = ref<Set<string>>(new Set())
  const titles = ref<InvoiceTitle[]>([])
  const userAgent = ref('')

  type AppliedAmountResult = {
    groups: MergeGroup[]
    resolvedCount: number
    unresolvedCount: number
    unresolvedOrderIds: string[]
  }

  type BatchOrderMeta = {
    orderAmountQuery: string
    originalOrderInfo: unknown
    resolvedAmount?: string | null
  }

  function mergeGroupsByOrgName(groups: MergeGroup[]): MergeGroup[] {
    const merged = new Map<string, MergeGroup>()

    groups.forEach((group) => {
      const key = group.orgName
      const existing = merged.get(key)

      if (existing) {
        existing.orders.push(...group.orders)
        existing.total += group.total
        return
      }

      merged.set(key, {
        orgName: group.orgName,
        total: group.total,
        orders: [...group.orders],
      })
    })

    return Array.from(merged.values())
  }

  function saveMergeGroupCache(groups: MergeGroup[]) {
    localStorage.setItem(MERGE_GROUP_CACHE_KEY, JSON.stringify(groups))
  }

  function readMergeGroupCache(): MergeGroup[] | null {
    const cached = localStorage.getItem(MERGE_GROUP_CACHE_KEY)
    if (!cached) {
      return null
    }

    try {
      const parsed = JSON.parse(cached)
      return Array.isArray(parsed) ? parsed as MergeGroup[] : null
    } catch (e) {
      console.warn('[DEBUG] invalid merge group cache:', e)
      localStorage.removeItem(MERGE_GROUP_CACHE_KEY)
      return null
    }
  }

  function buildResolvedAmountMap(batchOrders: BatchOrderSeed[]): Map<string, string> {
    const amountMap = new Map<string, string>()

    batchOrders.forEach((order) => {
      if (order.orderId && order.resolvedAmount) {
        amountMap.set(order.orderId, order.resolvedAmount)
      }
    })

    console.log(`[DEBUG] buildResolvedAmountMap candidates=${batchOrders.length} matched=${amountMap.size}`)
    return amountMap
  }

  function buildBatchOrderMetaMap(batchOrders: BatchOrderSeed[]): Map<string, BatchOrderMeta> {
    const metaMap = new Map<string, BatchOrderMeta>()

    batchOrders.forEach((order) => {
      if (!order.orderId) {
        return
      }

      metaMap.set(order.orderId, {
        orderAmountQuery: order.orderAmountQuery,
        originalOrderInfo: order.originalOrderInfo,
        resolvedAmount: order.resolvedAmount,
      })
    })

    return metaMap
  }

  function applyResolvedAmounts(
    groups: MergeGroup[],
    amountMap: Map<string, string>,
    metaMap: Map<string, BatchOrderMeta>,
  ): AppliedAmountResult {
    let resolvedCount = 0
    let unresolvedCount = 0
    const unresolvedOrderIds: string[] = []

    const nextGroups = groups.map((group) => {
      const orders = group.orders.map((order: MergeOrder) => {
        const orderId = typeof order.orderId === 'string' ? order.orderId : ''
        const resolvedAmount = orderId ? amountMap.get(orderId) : undefined
        const meta = orderId ? metaMap.get(orderId) : undefined
        const nextOrder = resolvedAmount
          ? {
              ...order,
              resolvedAmount,
              amountResolved: true,
              originalOrderInfo: meta?.originalOrderInfo ?? order.originalOrderInfo,
              orderAmountQuery: meta?.orderAmountQuery ?? order.orderAmountQuery,
            }
          : {
              ...order,
              amountResolved: false,
              originalOrderInfo: meta?.originalOrderInfo ?? order.originalOrderInfo,
              orderAmountQuery: meta?.orderAmountQuery ?? order.orderAmountQuery,
            }

        if (isMergeOrderAmountResolved(nextOrder)) {
          resolvedCount += 1
        } else {
          unresolvedCount += 1
          unresolvedOrderIds.push(orderId || '(missing-order-id)')
        }

        return nextOrder
      })

      return {
        ...group,
        orders,
        total: orders.reduce((sum, order) => sum + getMergeOrderAmount(order), 0),
      }
    })

    return {
      groups: nextGroups,
      resolvedCount,
      unresolvedCount,
      unresolvedOrderIds,
    }
  }

  async function initCookie() {
    const saved = await getCookie()
    if (saved) {
      cookie.value = saved
      isLoggedIn.value = true
    }
  }

  async function setCookie(newCookie: string) {
    await saveCookie(newCookie)
    cookie.value = newCookie
    isLoggedIn.value = true
  }

  async function logout() {
    await clearCookie()
  }

  async function loadTitles() {
    try {
      titles.value = await getTitles()
    } catch (e) {
      console.error('Failed to load titles:', e)
      titles.value = []
    }
  }

  async function saveTitlesData(newTitles: InvoiceTitle[]) {
    titles.value = newTitles
    await saveTitles(JSON.stringify(newTitles))
  }

  async function loadUserAgent() {
    try {
      userAgent.value = await getUserAgent()
    } catch (e) {
      console.error('Failed to load user agent:', e)
      userAgent.value = ''
    }
  }

  async function saveUserAgentData(nextUserAgent: string) {
    userAgent.value = await saveUserAgent(nextUserAgent)
  }

  async function resetUserAgentData() {
    userAgent.value = await resetUserAgent()
  }

  async function clearCookie() {
    await deleteCookie()
    cookie.value = null
    isLoggedIn.value = false
    invoices.value = []
    mergeGroups.value = []
    hasFetchedMergeGroups.value = false
    hasLoadedMergeGroupCache.value = false
    invoicePage.value = 1
    invoiceHasMore.value = true
    invoiceSeen.value = new Set()
  }

  async function loadInvoices() {
    loading.value = true
    loadingMore.value = false
    error.value = null
    invoicePage.value = 1
    invoiceHasMore.value = true
    invoiceSeen.value = new Set()
    invoices.value = []

    try {
      await loadNextPage()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  async function loadMoreInvoices() {
    if (loadingMore.value || !invoiceHasMore.value || loading.value) return
    loadingMore.value = true
    error.value = null

    try {
      await loadNextPage()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loadingMore.value = false
    }
  }

  async function loadNextPage() {
    console.log(`[DEBUG] loadNextPage page=${invoicePage.value}`)
    const pageResults = await fetchInvoices(invoicePage.value)
    console.log(`[DEBUG] loadNextPage got ${pageResults.length} items`)

    if (pageResults.length === 0) {
      invoiceHasMore.value = false
      return
    }

    for (const item of pageResults) {
      invoices.value.push(item)
    }

    console.log(`[DEBUG] loadNextPage total invoices=${invoices.value.length}`)

    invoicePage.value++
  }

  async function loadMergeGroups() {
    loading.value = true
    error.value = null
    try {
      console.log('[DEBUG] fetchBatchOrders start')
      const batchOrders = await fetchBatchOrders()
      console.log('[DEBUG] fetchBatchOrders result count:', batchOrders.length)
      console.log('[DEBUG] fetchBatchOrders sample:', JSON.stringify(batchOrders.slice(0, 2)))

      if (batchOrders.length === 0) {
        mergeGroups.value = []
        hasFetchedMergeGroups.value = true
        saveMergeGroupCache(mergeGroups.value)
        return
      }

      const resolvedAmountMap = buildResolvedAmountMap(batchOrders)
      const batchOrderMetaMap = buildBatchOrderMetaMap(batchOrders)
      const originalOrderInfos = batchOrders.map((order) => order.originalOrderInfo)
      const BATCH_SIZE = 40
      const allGroups: MergeGroup[] = []

      for (let i = 0; i < originalOrderInfos.length; i += BATCH_SIZE) {
        const batch = originalOrderInfos.slice(i, i + BATCH_SIZE)
        const orderListJson = encodeURIComponent(JSON.stringify(batch))
        console.log(`[DEBUG] checkMerge batch ${i / BATCH_SIZE + 1}, orders: ${batch.length}`)
        const groups = await checkMerge(orderListJson)
        console.log(`[DEBUG] checkMerge result count:`, groups.length)
        allGroups.push(...groups)

        await new Promise((r) => setTimeout(r, 800))
      }

      console.log('[DEBUG] allGroups total:', allGroups.length)
      const applied = applyResolvedAmounts(allGroups, resolvedAmountMap, batchOrderMetaMap)
      console.log(`[DEBUG] amount resolution summary candidates=${batchOrders.length} amountApiMatched=${resolvedAmountMap.size} resolvedOrders=${applied.resolvedCount} unresolvedOrders=${applied.unresolvedCount}`)
      if (applied.unresolvedOrderIds.length > 0) {
        console.warn('[DEBUG] unresolved orderIds:', applied.unresolvedOrderIds)
      }
      mergeGroups.value = mergeGroupsByOrgName(applied.groups)
      saveMergeGroupCache(mergeGroups.value)
      hasFetchedMergeGroups.value = true
      hasLoadedMergeGroupCache.value = true
      console.log(`[DEBUG] loadMergeGroups done groups=${mergeGroups.value.length}`)
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e)
      console.error('[DEBUG] loadMergeGroups error:', error.value)
      hasFetchedMergeGroups.value = true
    } finally {
      loading.value = false
    }
  }

  function loadCachedMergeGroups() {
    error.value = null
    const cachedGroups = readMergeGroupCache()
    hasLoadedMergeGroupCache.value = true
    hasFetchedMergeGroups.value = true

    if (!cachedGroups) {
      mergeGroups.value = []
      return false
    }

    mergeGroups.value = cachedGroups
    return true
  }

  function removeMergedOrders(exchangedOrders: MergeOrder[]) {
    const exchangedIds = new Set(
      exchangedOrders
        .map((order) => (typeof order.orderId === 'string' ? order.orderId : ''))
        .filter(Boolean),
    )

    if (exchangedIds.size === 0) {
      return
    }

    mergeGroups.value = mergeGroups.value
      .map((group) => {
        const orders = group.orders.filter((order) => {
          const orderId = typeof order.orderId === 'string' ? order.orderId : ''
          return !exchangedIds.has(orderId)
        })

        return {
          ...group,
          orders,
          total: orders.reduce((sum, order) => sum + getMergeOrderAmount(order), 0),
        }
      })
      .filter((group) => group.orders.length > 0)
  }

  return {
    isLoggedIn,
    cookie,
    invoices,
    mergeGroups,
    hasFetchedMergeGroups,
    hasLoadedMergeGroupCache,
    loading,
    loadingMore,
    invoiceHasMore,
    error,
    initCookie,
    setCookie,
    logout,
    clearCookie,
    loadInvoices,
    loadMoreInvoices,
    loadMergeGroups,
    loadCachedMergeGroups,
    removeMergedOrders,
    titles,
    loadTitles,
    saveTitlesData,
    userAgent,
    loadUserAgent,
    saveUserAgentData,
    resetUserAgentData,
  }
})
