<template>
  <header class="header">
    <div class="header-left">
      <h1 class="logo" @click="goHome">京东发票管理</h1>
    </div>
    <nav class="header-nav">
      <router-link v-if="store.isLoggedIn" to="/" class="nav-link">发票列表</router-link>
      <router-link v-if="store.isLoggedIn" to="/exchange" class="nav-link">换开发票</router-link>
      <router-link v-if="store.isLoggedIn" to="/apply-records" class="nav-link">申请记录</router-link>
      <router-link to="/settings" class="nav-link">设置</router-link>
      <button v-if="store.isLoggedIn" class="login-btn logout" @click="handleLogout">退出登录</button>
      <router-link v-else-if="$route.path !== '/login'" to="/login" class="login-btn">登录</router-link>
    </nav>
  </header>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'

const store = useAppStore()
const router = useRouter()

function goHome() {
  if (store.isLoggedIn) {
    router.push('/')
  } else {
    router.push('/login')
  }
}

async function handleLogout() {
  await store.logout()
  router.push('/login')
}
</script>

<style scoped>
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  height: 56px;
  background: white;
  border-bottom: 1px solid #eee;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
}

.logo {
  font-size: 18px;
  font-weight: 600;
  color: #e2231a;
  cursor: pointer;
}

.header-nav {
  display: flex;
  align-items: center;
  gap: 16px;
}

.nav-link {
  text-decoration: none;
  color: #666;
  font-size: 14px;
  padding: 6px 12px;
  border-radius: 4px;
  transition: all 0.2s;
}

.nav-link:hover,
.nav-link.router-link-active {
  color: #e2231a;
  background-color: #fff5f5;
}

.login-btn {
  padding: 6px 16px;
  font-size: 14px;
  border-radius: 4px;
  cursor: pointer;
  text-decoration: none;
  border: none;
  background-color: #e2231a;
  color: white;
  transition: all 0.2s;
}

.login-btn:hover {
  background-color: #c41e1a;
}

.login-btn.logout {
  background-color: #f5f5f5;
  color: #666;
  border: 1px solid #ddd;
}

.login-btn.logout:hover {
  background-color: #eee;
  color: #333;
}
</style>
