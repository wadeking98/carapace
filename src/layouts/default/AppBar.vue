<template>
  <v-app-bar elevation="3">
    <v-tabs v-if="$vuetify.display.mdAndUp">
      <v-tab prepend-icon="mdi-home" to="/">Home</v-tab>
      <v-tab prepend-icon="mdi-message" to="/messages">Messages</v-tab>
      <v-tab prepend-icon="mdi-console" to="/shells">Shells</v-tab>
    </v-tabs>
    <template v-if="$vuetify.display.smAndDown" v-slot:prepend>
      <v-app-bar-nav-icon @click="drawer = !drawer" />
    </template>
  </v-app-bar>
  <v-navigation-drawer v-if="$vuetify.display.smAndDown" v-model="drawer" app>
    <v-list-item class="mt-5" prepend-icon="mdi-home" title="Home" to="/"/>
    <v-list-item prepend-icon="mdi-message" title="Messages" to="/messages"/>
    <v-list-item prepend-icon="mdi-console" title="Shells" to="/shells"/>
  </v-navigation-drawer>
</template>

<script lang="ts" setup>
import { ref } from 'vue';
const drawer = ref(false);
</script>
