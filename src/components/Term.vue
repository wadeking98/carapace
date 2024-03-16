<template>
    <div style="display: flex; align-items: center; justify-self: center; flex-direction: column; width: 80%; height: 80vh; ">
        <v-toolbar :title="props.shellName">
            <template v-slot:prepend>
                <slot name="back"></slot>
            </template>
        </v-toolbar>
        <div ref="terminalContainer" style="flex-grow: 1; width: 100%;"></div>
    </div>
</template>
<script lang="ts" setup>
import "xterm/css/xterm.css";
import { Terminal } from "xterm";
import { ref } from "vue";
import { onMounted } from "vue";
import { FitAddon } from 'xterm-addon-fit';

const props = defineProps({
    shellName: String
})

const terminalContainer = ref(null);

onMounted(() => {
    if (!terminalContainer.value) {
        return;
    }
    const terminal = new Terminal();
    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(terminalContainer.value);
    terminal.write("$ ");
    fitAddon.fit();
});

</script>