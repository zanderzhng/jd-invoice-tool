<template>
  <div class="settings-view">
    <h2>设置</h2>

    <div class="section card">
      <h3 class="section-title">Cookie 管理</h3>

      <div v-if="store.cookie" class="cookie-status">
        <span class="status-dot logged"></span>
        <span>当前已保存 Cookie</span>
      </div>
      <div v-else class="cookie-status">
        <span class="status-dot unlogged"></span>
        <span>未保存 Cookie</span>
      </div>

      <CookieInput v-model="cookieText" />

      <div class="btn-group">
        <button class="btn btn-primary" :disabled="!cookieText || store.loading" @click="handleSave">
          保存 Cookie
        </button>
        <button class="btn btn-secondary" :disabled="!store.cookie" @click="handleDelete">
          删除 Cookie
        </button>
      </div>

      <div v-if="message" :class="['msg', messageType]">
        {{ message }}
      </div>
    </div>

    <div class="section card">
      <h3 class="section-title">请求设置</h3>

      <label class="field">
        <span class="field-label">User-Agent</span>
        <textarea
          v-model="userAgentText"
          class="input ua-input"
          rows="3"
          placeholder="请输入用于请求京东接口的 User-Agent"
        ></textarea>
      </label>

      <div class="btn-group">
        <button class="btn btn-primary" :disabled="!userAgentText.trim() || store.loading" @click="handleSaveUserAgent">
          保存 UA
        </button>
        <button class="btn btn-secondary" :disabled="store.loading" @click="handleResetUserAgent">
          恢复默认 UA
        </button>
      </div>

      <div v-if="uaMessage" :class="['msg', uaMessageType]">
        {{ uaMessage }}
      </div>
    </div>

    <div class="section card">
      <h3 class="section-title">发票抬头管理</h3>
      <TitleManager :titles="store.titles" @save="handleSaveTitles" />
    </div>

    <div class="section card">
      <h3 class="section-title">关于</h3>
      <p class="about-text">{{ aboutText }}</p>
      <p class="about-text">基于 Tauri + Vue3 构建</p>
    </div>

    <LoadingOverlay :visible="store.loading" text="处理中..." />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { getAppName, getAppVersion } from '@/api'
import type { InvoiceTitle } from '@/types/title'
import CookieInput from '@/components/CookieInput.vue'
import LoadingOverlay from '@/components/LoadingOverlay.vue'
import TitleManager from '@/components/TitleManager.vue'

const store = useAppStore()
const cookieText = ref(store.cookie || '')
const userAgentText = ref(store.userAgent)
const appName = ref('京东发票管理工具')
const appVersion = ref('')
const message = ref('')
const messageType = ref<'success' | 'error'>('success')
const uaMessage = ref('')
const uaMessageType = ref<'success' | 'error'>('success')
const aboutText = computed(() => (appVersion.value ? `${appName.value} v${appVersion.value}` : appName.value))

onMounted(async () => {
  void store.loadTitles()
  void loadAppInfo()
  await store.loadUserAgent()
  userAgentText.value = store.userAgent
})

async function loadAppInfo() {
  try {
    const [name, version] = await Promise.all([getAppName(), getAppVersion()])
    appName.value = name
    appVersion.value = version
  } catch (e) {
    console.error('Failed to load app info:', e)
  }
}

async function handleSave() {
  if (!cookieText.value.trim()) return
  store.loading = true
  message.value = ''
  try {
    await store.setCookie(cookieText.value.trim())
    messageType.value = 'success'
    message.value = 'Cookie 保存成功'
  } catch (e: unknown) {
    messageType.value = 'error'
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    store.loading = false
  }
}

async function handleDelete() {
  store.loading = true
  message.value = ''
  try {
    await store.clearCookie()
    cookieText.value = ''
    messageType.value = 'success'
    message.value = 'Cookie 已删除'
  } catch (e: unknown) {
    messageType.value = 'error'
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    store.loading = false
  }
}

async function handleSaveUserAgent() {
  const nextUserAgent = userAgentText.value.trim()
  if (!nextUserAgent) return
  store.loading = true
  uaMessage.value = ''
  try {
    await store.saveUserAgentData(nextUserAgent)
    userAgentText.value = store.userAgent
    uaMessageType.value = 'success'
    uaMessage.value = 'UA 保存成功'
  } catch (e: unknown) {
    uaMessageType.value = 'error'
    uaMessage.value = e instanceof Error ? e.message : String(e)
  } finally {
    store.loading = false
  }
}

async function handleResetUserAgent() {
  store.loading = true
  uaMessage.value = ''
  try {
    await store.resetUserAgentData()
    userAgentText.value = store.userAgent
    uaMessageType.value = 'success'
    uaMessage.value = 'UA 已恢复默认'
  } catch (e: unknown) {
    uaMessageType.value = 'error'
    uaMessage.value = e instanceof Error ? e.message : String(e)
  } finally {
    store.loading = false
  }
}

async function handleSaveTitles(titles: InvoiceTitle[]) {
  await store.saveTitlesData(titles)
}
</script>

<style scoped>
.settings-view h2 {
  font-size: 20px;
  font-weight: 600;
  margin-bottom: 20px;
}

.section {
  margin-bottom: 16px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 16px;
  color: #333;
}

.cookie-status {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: #666;
  margin-bottom: 16px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.status-dot.logged {
  background-color: #52c41a;
}

.status-dot.unlogged {
  background-color: #fa8c16;
}

.field {
  display: grid;
  gap: 6px;
}

.field-label {
  font-size: 13px;
  color: #666;
}

.ua-input {
  width: 100%;
  min-height: 78px;
  resize: vertical;
  line-height: 1.5;
}

.btn-group {
  display: flex;
  gap: 10px;
  margin-top: 12px;
}

.msg {
  margin-top: 12px;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
}

.msg.success {
  background-color: #f6ffed;
  color: #52c41a;
  border: 1px solid #b7eb8f;
}

.msg.error {
  background-color: #fff2f0;
  color: #e2231a;
  border: 1px solid #ffccc7;
}

.about-text {
  font-size: 14px;
  color: #999;
  margin-bottom: 4px;
}
</style>
