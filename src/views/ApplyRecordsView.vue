<template>
  <div class="apply-records" ref="scrollContainer">
    <div class="page-header">
      <div>
        <h2>申请记录</h2>
        <p class="subtitle">批量获取换开发票记录，并自动下载已换开的 PDF。</p>
      </div>

      <div class="actions">
        <label class="date-field">
          <span>获取到</span>
          <input v-model="cutoffDate" type="date" class="input date-input" :max="todayText" />
        </label>
        <button class="btn btn-primary" :disabled="fetching || !cutoffDate" @click="startBatchFetch">
          {{ fetching ? '获取中...' : '开始获取' }}
        </button>
      </div>
    </div>

    <div v-if="error" class="error-msg">{{ error }}</div>

    <div v-if="records.length > 0" class="summary-bar">
      <span>已获取 {{ records.length }} 条</span>
      <span>已换开 {{ completedCount }} 条</span>
      <span>已下载 {{ downloadedCount }} 个 PDF</span>
      <span v-if="fetching">{{ progressText }}</span>
    </div>

    <div v-if="records.length === 0 && !fetching" class="empty">
      <p>选择日期后开始获取申请记录</p>
    </div>

    <div v-else class="record-list">
      <article v-for="record in records" :key="record.orderId" class="record-card card">
        <div class="record-main">
          <div class="record-title-row">
            <strong class="record-title">{{ productText(record) }}</strong>
            <span class="status-badge" :class="statusClass(record.status)">{{ record.status }}</span>
          </div>

          <div class="record-grid">
            <div>
              <span class="label">订单号</span>
              <span>{{ record.orderId }}</span>
            </div>
            <div>
              <span class="label">发票号</span>
              <span>{{ invoiceNos(record) }}</span>
            </div>
            <div>
              <span class="label">开票人</span>
              <span>{{ record.detail?.venderName || record.venderName || '-' }}</span>
            </div>
            <div>
              <span class="label">发票抬头</span>
              <span>{{ record.detail?.invoiceTitle || record.invoiceTitle || '-' }}</span>
            </div>
            <div>
              <span class="label">申请时间</span>
              <span>{{ record.detail?.applyTime || record.applyTime || '-' }}</span>
            </div>
            <div>
              <span class="label">开票时间</span>
              <span>{{ record.detail?.invoiceTime || '-' }}</span>
            </div>
            <div>
              <span class="label">金额</span>
              <span>¥{{ record.detail?.invoiceAmount || record.amount || '-' }}</span>
            </div>
            <div>
              <span class="label">类型</span>
              <span>{{ invoiceMeta(record) }}</span>
            </div>
            <div>
              <span class="label">下载状态</span>
              <span>{{ record.downloadStatusText }}</span>
            </div>
          </div>
        </div>

        <div class="record-actions">
          <button
            v-if="record.pdfPath"
            class="btn btn-secondary"
            type="button"
            @click="openDownloadedPdf(record)"
          >
            打开
          </button>
          <button
            class="btn btn-secondary"
            type="button"
            :disabled="fetching || !isCompleted(record)"
            @click="downloadOne(record)"
          >
            重新下载
          </button>
        </div>
      </article>
    </div>

    <LoadingOverlay :visible="fetching" :text="progressText || '正在获取申请记录...'" />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { downloadInvoice, fetchApplyRecordDetail, fetchApplyRecords, openBrowser } from '@/api'
import LoadingOverlay from '@/components/LoadingOverlay.vue'
import type { ApplyRecord, ApplyRecordDetail } from '@/types'

interface DisplayApplyRecord extends ApplyRecord {
  detail?: ApplyRecordDetail
  downloadStatus: 'pending' | 'skipped' | 'checking' | 'downloading' | 'downloaded' | 'failed'
  downloadStatusText: string
  pdfPath?: string
  pdfUrl?: string
}

interface ApplyRecordCachePayload {
  version: 1
  coveredTo: string | null
  records: CachedApplyRecord[]
}

interface CachedApplyRecord extends ApplyRecord {
  detail?: ApplyRecordDetail
  pdfPath?: string
  pdfUrl?: string
  cachedAt?: string
}

const APPLY_RECORD_CACHE_KEY = 'jd-invoice-tool:apply-records:v1'
const todayText = new Date().toISOString().slice(0, 10)
const cutoffDate = ref(todayText)
const records = ref<DisplayApplyRecord[]>([])
const fetching = ref(false)
const error = ref<string | null>(null)
const progressText = ref('')
const scrollContainer = ref<HTMLElement | null>(null)
const downloadedPdfUrls = new Set<string>()

const completedCount = computed(() => records.value.filter((record) => isCompleted(record)).length)
const downloadedCount = computed(() => records.value.filter((record) => record.downloadStatus === 'downloaded').length)

function isCompleted(record: ApplyRecord): boolean {
  return record.status.includes('已换开') || record.reqState === 2 || record.reqState === 3
}

function productText(record: ApplyRecord): string {
  const names = record.products
    .map((product) => product.name || product.skuName || '')
    .filter(Boolean)

  return names.length > 0 ? names.join('、') : '换开发票记录'
}

function invoiceMeta(record: DisplayApplyRecord): string {
  const detail = record.detail
  const invoiceType = detail?.invoiceType || record.invoiceType
  const content = detail?.invoiceContentName || record.invoiceContentName
  const titleType = detail?.titleType || record.titleType

  return [invoiceType, content, titleType].filter(Boolean).join(' - ') || '-'
}

function invoiceNos(record: DisplayApplyRecord): string {
  const nos = record.detail?.files
    .map((file) => file.ivcNo?.trim())
    .filter((ivcNo): ivcNo is string => Boolean(ivcNo))

  if (!nos || nos.length === 0) {
    return '-'
  }

  return Array.from(new Set(nos)).join('、')
}

function statusClass(status: string): string {
  if (status.includes('已换开') || status.includes('已补开')) {
    return 'status-success'
  }
  if (status.includes('驳回') || status.includes('失败')) {
    return 'status-danger'
  }
  if (status.includes('中') || status.includes('申请')) {
    return 'status-warning'
  }
  return 'status-default'
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}

function randomDelay(min: number, max: number): number {
  return min + Math.floor(Math.random() * (max - min + 1))
}

function isBeforeCutoff(record: ApplyRecord): boolean {
  if (!record.applyTime) {
    return false
  }

  return record.applyTime.slice(0, 10) < cutoffDate.value
}

function recordDate(record: ApplyRecord): string {
  return (record.applyTime || '').slice(0, 10)
}

function recordTimeValue(record: ApplyRecord & { detail?: ApplyRecordDetail }): number {
  const timeText = record.detail?.applyTime || record.applyTime || ''
  const time = new Date(timeText.replace(/-/g, '/')).getTime()
  return Number.isFinite(time) ? time : 0
}

function sortRecordsByTime<T extends ApplyRecord & { detail?: ApplyRecordDetail }>(items: T[]): T[] {
  return [...items].sort((a, b) => recordTimeValue(b) - recordTimeValue(a))
}

function minDateText(a: string | null, b: string): string {
  if (!a) {
    return b
  }

  return a < b ? a : b
}

function readApplyRecordCache(): ApplyRecordCachePayload {
  const fallback: ApplyRecordCachePayload = {
    version: 1,
    coveredTo: null,
    records: [],
  }
  const cached = localStorage.getItem(APPLY_RECORD_CACHE_KEY)
  if (!cached) {
    return fallback
  }

  try {
    const parsed = JSON.parse(cached) as Partial<ApplyRecordCachePayload>
    if (parsed.version !== 1 || !Array.isArray(parsed.records)) {
      return fallback
    }

    return {
      version: 1,
      coveredTo: typeof parsed.coveredTo === 'string' ? parsed.coveredTo : null,
      records: sortRecordsByTime(parsed.records.filter((record) => typeof record.orderId === 'string')),
    }
  } catch (e) {
    console.warn('[apply-records] invalid cache:', e)
    localStorage.removeItem(APPLY_RECORD_CACHE_KEY)
    return fallback
  }
}

function cacheCanCover(cached: ApplyRecordCachePayload): boolean {
  return Boolean(cached.coveredTo && cached.coveredTo <= cutoffDate.value)
}

function hydrateCachedRecord(record: CachedApplyRecord): DisplayApplyRecord {
  const displayRecord = normalizeRecord(record)
  displayRecord.detail = record.detail
  displayRecord.pdfPath = record.pdfPath
  displayRecord.pdfUrl = record.pdfUrl

  if (record.pdfPath) {
    displayRecord.downloadStatus = 'downloaded'
    displayRecord.downloadStatusText = '已下载（缓存）'
  } else if (isCompleted(displayRecord)) {
    displayRecord.downloadStatus = 'pending'
    displayRecord.downloadStatusText = '来自缓存，未下载'
  }

  return displayRecord
}

function mergeFetchedWithCache(record: ApplyRecord, cachedRecord?: CachedApplyRecord): DisplayApplyRecord {
  const displayRecord = normalizeRecord(record)
  if (!cachedRecord) {
    return displayRecord
  }

  displayRecord.detail = cachedRecord.detail
  displayRecord.pdfPath = cachedRecord.pdfPath
  displayRecord.pdfUrl = cachedRecord.pdfUrl

  if (cachedRecord.pdfPath) {
    displayRecord.downloadStatus = 'downloaded'
    displayRecord.downloadStatusText = '已下载（缓存）'
  }

  return displayRecord
}

function buildCachedRecord(record: DisplayApplyRecord): CachedApplyRecord {
  return {
    orderId: record.orderId,
    status: record.status,
    reqState: record.reqState,
    reqType: record.reqType,
    applyTime: record.detail?.applyTime || record.applyTime,
    invoiceTitle: record.detail?.invoiceTitle || record.invoiceTitle,
    invoiceType: record.detail?.invoiceType || record.invoiceType,
    invoiceContentName: record.detail?.invoiceContentName || record.invoiceContentName,
    titleType: record.detail?.titleType || record.titleType,
    amount: record.detail?.invoiceAmount || record.amount,
    venderName: record.detail?.venderName || record.venderName,
    tagStr: record.tagStr,
    products: record.products,
    detail: record.detail,
    pdfPath: record.pdfPath,
    pdfUrl: record.pdfUrl,
    cachedAt: new Date().toISOString(),
  }
}

function saveApplyRecordCache(currentRecords: DisplayApplyRecord[], coveredTo?: string) {
  const previous = readApplyRecordCache()
  const byOrderId = new Map<string, CachedApplyRecord>()

  previous.records.forEach((record) => {
    byOrderId.set(record.orderId, record)
  })
  currentRecords.forEach((record) => {
    byOrderId.set(record.orderId, buildCachedRecord(record))
  })

  const payload: ApplyRecordCachePayload = {
    version: 1,
    coveredTo: coveredTo ? minDateText(previous.coveredTo, coveredTo) : previous.coveredTo,
    records: sortRecordsByTime(Array.from(byOrderId.values())),
  }

  localStorage.setItem(APPLY_RECORD_CACHE_KEY, JSON.stringify(payload))
}

async function appendCachedTail(cached: ApplyRecordCachePayload, seen: Set<string>) {
  for (const cachedRecord of cached.records) {
    if (seen.has(cachedRecord.orderId)) {
      continue
    }

    if (recordDate(cachedRecord) && recordDate(cachedRecord) < cutoffDate.value) {
      break
    }

    seen.add(cachedRecord.orderId)
    records.value.push(hydrateCachedRecord(cachedRecord))
    await scrollToBottom()
  }
}

function makeFilename(record: DisplayApplyRecord): string {
  const time = (record.detail?.invoiceTime || record.applyTime || '').slice(0, 10).replace(/-/g, '')
  const title = (record.detail?.invoiceTitle || record.invoiceTitle || 'invoice').replace(/[\\/:*?"<>|]/g, '_').slice(0, 48)
  const amount = (record.detail?.invoiceAmount || record.amount || '').replace(/[^\d.]/g, '')
  const ivcNo = record.detail?.files.find((file) => file.ivcNo)?.ivcNo

  return `jd_apply_${time || 'unknown'}_${title}_${amount || record.orderId}_${ivcNo || record.orderId}.pdf`
}

async function scrollToBottom() {
  await nextTick()
  const el = scrollContainer.value
  if (el) {
    el.scrollTop = el.scrollHeight
  }
}

function normalizeRecord(record: ApplyRecord): DisplayApplyRecord {
  return {
    ...record,
    downloadStatus: isCompleted(record) ? 'pending' : 'skipped',
    downloadStatusText: isCompleted(record) ? '等待下载' : '未换开完成，跳过下载',
  }
}

async function startBatchFetch() {
  if (fetching.value || !cutoffDate.value) {
    return
  }

  fetching.value = true
  error.value = null
  progressText.value = ''
  records.value = []
  downloadedPdfUrls.clear()

  try {
    const cached = readApplyRecordCache()
    const cachedByOrderId = new Map(cached.records.map((record) => [record.orderId, record]))
    cached.records.forEach((record) => {
      if (record.pdfUrl) {
        downloadedPdfUrls.add(record.pdfUrl)
      }
    })

    let page = 1
    let shouldStop = false
    const seen = new Set<string>()

    while (!shouldStop) {
      progressText.value = `正在获取第 ${page} 页`
      const pageRecords = await fetchApplyRecords(page)

      if (pageRecords.length === 0) {
        break
      }

      for (const record of pageRecords) {
        if (isBeforeCutoff(record)) {
          shouldStop = true
          continue
        }

        if (seen.has(record.orderId)) {
          continue
        }

        seen.add(record.orderId)
        const cachedRecord = cachedByOrderId.get(record.orderId)
        const displayRecord = mergeFetchedWithCache(record, cachedRecord)
        records.value.push(displayRecord)
        await scrollToBottom()

        if (cachedRecord && cacheCanCover(cached)) {
          progressText.value = '已访问到历史缓存，正在读取本地缓存'
          await appendCachedTail(cached, seen)
          shouldStop = true
          break
        }

        if (isCompleted(displayRecord) && !displayRecord.pdfPath) {
          await downloadOne(displayRecord)
          await sleep(randomDelay(700, 1400))
        }
      }

      saveApplyRecordCache(records.value)
      page += 1
      if (!shouldStop) {
        progressText.value = `第 ${page - 1} 页完成，稍后继续`
        await sleep(randomDelay(1200, 2600))
      }
    }

    saveApplyRecordCache(records.value, cutoffDate.value)
    progressText.value = `获取完成，共 ${records.value.length} 条`
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    fetching.value = false
  }
}

async function downloadOne(record: DisplayApplyRecord) {
  record.downloadStatus = 'checking'
  record.downloadStatusText = '查看详情中'

  try {
    const detail = await fetchApplyRecordDetail(record.orderId, record.tagStr)
    record.detail = detail
    const file = detail.files.find((item) => item.pdfUrl || item.fileUrl)
    const pdfUrl = file?.pdfUrl || file?.fileUrl || ''

    if (!pdfUrl) {
      record.downloadStatus = 'failed'
      record.downloadStatusText = '未找到 PDF'
      return
    }

    record.pdfUrl = pdfUrl
    if (downloadedPdfUrls.has(pdfUrl)) {
      record.downloadStatus = 'downloaded'
      record.downloadStatusText = '同一 PDF 已下载'
      return
    }

    record.downloadStatus = 'downloading'
    record.downloadStatusText = '下载 PDF 中'
    const path = await downloadInvoice(pdfUrl, makeFilename(record))
    downloadedPdfUrls.add(pdfUrl)
    record.pdfPath = path
    record.downloadStatus = 'downloaded'
    record.downloadStatusText = '已下载'
    saveApplyRecordCache(records.value)
  } catch (e: unknown) {
    record.downloadStatus = 'failed'
    record.downloadStatusText = e instanceof Error ? e.message : String(e)
    saveApplyRecordCache(records.value)
  }
}

async function openDownloadedPdf(record: DisplayApplyRecord) {
  if (!record.pdfPath) {
    return
  }

  await openBrowser(record.pdfPath)
}
</script>

<style scoped>
.apply-records {
  height: calc(100vh - 96px);
  overflow-y: auto;
  padding: 0 20px 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
  margin-bottom: 16px;
  position: sticky;
  top: 0;
  background: #f5f5f5;
  padding: 10px 0;
  z-index: 1;
}

.page-header h2 {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.subtitle {
  margin-top: 6px;
  color: #666;
  font-size: 13px;
}

.actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.date-field {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #666;
  font-size: 13px;
  white-space: nowrap;
}

.date-input {
  width: 150px;
}

.summary-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  padding: 10px 14px;
  background: #fffbe6;
  border: 1px solid #ffe58f;
  border-radius: 6px;
  color: #666;
  font-size: 13px;
  margin-bottom: 12px;
}

.empty {
  text-align: center;
  padding: 60px 20px;
  color: #999;
}

.record-list {
  display: grid;
  gap: 12px;
}

.record-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 16px;
  align-items: start;
}

.record-main {
  min-width: 0;
}

.record-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
}

.record-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 14px;
}

.status-badge {
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 10px;
  white-space: nowrap;
}

.status-success {
  background-color: #f6ffed;
  color: #52c41a;
  border: 1px solid #b7eb8f;
}

.status-warning {
  background-color: #fffbe6;
  color: #faad14;
  border: 1px solid #ffe58f;
}

.status-danger {
  background-color: #fff2f0;
  color: #ff4d4f;
  border: 1px solid #ffccc7;
}

.status-default {
  background-color: #fafafa;
  color: #999;
  border: 1px solid #d9d9d9;
}

.record-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
  gap: 8px 14px;
  color: #333;
  font-size: 13px;
}

.record-grid div {
  display: flex;
  gap: 8px;
  min-width: 0;
}

.label {
  flex: 0 0 62px;
  color: #999;
}

.record-grid span:last-child {
  min-width: 0;
  overflow-wrap: anywhere;
}

.record-actions {
  display: flex;
  gap: 8px;
}

@media (max-width: 720px) {
  .page-header,
  .actions,
  .record-card,
  .record-actions {
    grid-template-columns: 1fr;
    flex-direction: column;
    align-items: stretch;
  }

  .date-field {
    align-items: flex-start;
  }

  .date-input,
  .actions .btn,
  .record-actions .btn {
    width: 100%;
  }
}
</style>
