<template>
    <v-window style="height: 100%; width: 100%;" v-model="pane">
        <v-window-item value="0">
            <v-list style="width: 80%; height: auto;">
                <v-list-item style="margin-top: 10px;" v-for="(shell, i) in shells" :key="i" @click="() => {
                    currentShellId = shell.id;
                    pane++;
                }">
                    <v-list-item-title>{{ shell.name }}</v-list-item-title>
                </v-list-item>
            </v-list>

        </v-window-item>
        <v-window-item value="1">
            <Term v-if="currentShell" :shellName="currentShell.name">
                <template v-slot:back>
                    <v-btn icon="mdi-chevron-left" @click="pane--"></v-btn>
                </template>
            </Term>
        </v-window-item>
    </v-window>
</template>
<script lang="ts" setup>
import Term from "@/components/Term.vue";
import { computed } from "vue";
import { ref } from "vue";
const pane = ref(0);
const currentShellId = ref(0);
const shells = ref([
    {
        name: "shell name 1",
        id: 0
    },
    {
        name: "shell name 2",
        id: 1
    },
    {
        name: "shell name 3",
        id: 3
    }
]);
const currentShell = computed(() => {
    return shells.value.find(shell => shell.id === currentShellId.value);
});
</script>
<style scoped>
.v-window-item {
    height: 100%;
    width: 100%;
    display: flex;
    align-items: center;
    flex-direction: column;
    flex-grow: 1;
    margin-top: 20px;
}
</style>