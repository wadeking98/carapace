<template>
  <v-container class="content">
    <h1>Home</h1>
    <v-textarea label="Public Key" ref="pubkeyWindow" v-model="pubkey" variant="outlined" readonly auto-grow
      :class="{ 'w-50': $vuetify.display.mdAndUp, 'w-75': $vuetify.display.smAndDown }" class="mt-15">
      <template v-slot:append-inner>
        <v-tooltip open-on-click :open-on-hover="false" v-model="showCopied" text="Copied">
          <template v-slot:activator="{ props }">
            <div @mouseleave="showCopied = false">
              <v-btn v-bind="props" icon="mdi-content-copy" @click="copyPubKey" variant="text" size="sm"></v-btn>
            </div>
          </template>
        </v-tooltip>
      </template>

    </v-textarea>
  </v-container>
</template>

<script lang="ts" setup>
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api'
const pubkey = ref("test");
invoke('get_pub_key').then(key => pubkey.value = key as string)

const showCopied = ref(false);

const copyPubKey = () => {
  showCopied.value = true
  navigator.clipboard.writeText(pubkey.value)
}
</script>

<style scoped>
.content {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: auto;
}
</style>
