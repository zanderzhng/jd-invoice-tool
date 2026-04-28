import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getName, getVersion } from '@tauri-apps/api/app'
import type {
  BatchOrderSeed,
  ApplyRecord,
  ApplyRecordDetail,
  InvoiceItem,
  InvoiceFile,
  MergeExchangeResult,
  MergeExchangeTitleInput,
  MergeGroup,
  MergeOrder,
} from '@/types'
import type { InvoiceTitle } from '@/types/title'

export async function openBrowser(url: string): Promise<void> {
  await invoke<string>('open_browser', { url })
}

export async function openLoginWindow(): Promise<void> {
  await invoke<string>('open_login_window')
}

export async function openLoginWindowClean(): Promise<void> {
  await invoke<string>('open_login_window_clean')
}

export async function closeLoginWindow(): Promise<void> {
  await invoke<string>('close_login_window')
}

export async function closeLoginWindowClean(): Promise<void> {
  await invoke<string>('close_login_window_clean')
}

export async function clearLoginWindowCookies(): Promise<void> {
  await invoke<string>('clear_login_window_cookies')
}

export async function clearLoginWindowCache(): Promise<void> {
  await invoke<string>('clear_login_window_cache')
}

export async function isLoginWindowOpen(): Promise<boolean> {
  return invoke<boolean>('is_login_window_open')
}

export async function isLoginWindowCleanOpen(): Promise<boolean> {
  return invoke<boolean>('is_login_window_clean_open')
}

export async function hasLoginSucceeded(): Promise<boolean> {
  return invoke<boolean>('has_login_succeeded')
}

export async function extractCookies(): Promise<string> {
  return invoke<string>('extract_cookies')
}

export async function extractCookiesClean(): Promise<string> {
  return invoke<string>('extract_cookies_clean')
}

export interface LoginWindowEvent {
  url: string
  logged_in: boolean
}

export async function onLoginSuccess(callback: (event: LoginWindowEvent) => void): Promise<UnlistenFn> {
  return listen<LoginWindowEvent>('login-success', (event) => {
    callback(event.payload)
  })
}

export async function saveCookie(cookie: string): Promise<string> {
  return invoke<string>('save_cookie', { cookie })
}

export async function getCookie(): Promise<string | null> {
  return invoke<string | null>('get_cookie')
}

export async function deleteCookie(): Promise<string> {
  return invoke<string>('delete_cookie')
}

export async function fetchInvoices(page: number): Promise<InvoiceItem[]> {
  const result = await invoke<InvoiceItem[]>('fetch_invoices', { page })
  return result
}

export async function fetchInvoiceDetail(orderId: string): Promise<InvoiceFile[]> {
  return invoke<InvoiceFile[]>('fetch_invoice_detail', { orderId })
}

export async function fetchBatchOrders(): Promise<BatchOrderSeed[]> {
  return invoke<BatchOrderSeed[]>('fetch_batch_orders')
}

export async function fetchApplyRecords(page: number): Promise<ApplyRecord[]> {
  return invoke<ApplyRecord[]>('fetch_apply_records', { page })
}

export async function fetchApplyRecordDetail(orderId: string, tagStr: string): Promise<ApplyRecordDetail> {
  return invoke<ApplyRecordDetail>('fetch_apply_record_detail', { orderId, tagStr })
}

export async function checkMerge(orderListJson: string): Promise<MergeGroup[]> {
  return invoke<MergeGroup[]>('check_merge', { orderListJson })
}

export async function fetchBatchOrderAmounts(orderAmountQueries: string[]): Promise<Record<string, string>> {
  return invoke<Record<string, string>>('fetch_batch_order_amounts', { orderAmountQueries })
}

export async function submitMergeExchange(
  selectedOrders: MergeOrder[],
  title: MergeExchangeTitleInput,
): Promise<MergeExchangeResult> {
  return invoke<MergeExchangeResult>('submit_merge_exchange', {
    selectedOrders,
    title,
  })
}

export async function downloadInvoice(url: string, filename: string): Promise<string> {
  return invoke<string>('download_invoice', { url, filename })
}

export async function getTitles(): Promise<InvoiceTitle[]> {
  return invoke<InvoiceTitle[]>('get_titles')
}

export async function saveTitles(titlesJson: string): Promise<string> {
  return invoke<string>('save_titles', { titlesJson })
}

export async function getAppVersion(): Promise<string> {
  return getVersion()
}

export async function getAppName(): Promise<string> {
  return getName()
}

export async function getUserAgent(): Promise<string> {
  return invoke<string>('get_user_agent')
}

export async function saveUserAgent(userAgent: string): Promise<string> {
  return invoke<string>('save_user_agent', { userAgent })
}

export async function resetUserAgent(): Promise<string> {
  return invoke<string>('reset_user_agent')
}
