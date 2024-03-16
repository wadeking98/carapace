<template>
    <v-row style="overflow-y: hidden;" no-gutters>
        <v-col cols="3" :xs="smAndNavCollapsed ? 1 : 3">
            <div style="height: 100%;">

                <v-list-item class="mt-5">
                    <template v-slot:prepend>
                        <v-list-item-icon class="mr-5">
                            <v-icon>mdi-account</v-icon>
                            <v-icon style="position: absolute; top: 3px; left: 3px;"
                                :style="`color: ${statuses.find(stat => stat.status === userStatus)?.color ?? 'grey'}`">mdi-circle-medium</v-icon>
                        </v-list-item-icon>
                    </template>
                    <template v-if="smAndNavCollapsed" v-slot:append>
                        <v-list-item-action>
                            <v-btn icon="mdi-chevron-right"></v-btn>
                        </v-list-item-action>
                    </template>
                    <template v-else v-slot:append>
                        <v-list-item-action>
                            <v-btn variant="text" icon="mdi-magnify" size="xs"></v-btn>
                        </v-list-item-action>
                        <v-list-item-action class="ml-5">
                            <v-btn variant="text" icon="mdi-square-edit-outline" size="xs"></v-btn>
                        </v-list-item-action>
                        <v-list-item-action class="ml-5">
                            <v-btn variant="text" icon="mdi-dots-vertical" size="xs"></v-btn>
                        </v-list-item-action>
                    </template>
                </v-list-item>
                <v-divider thickness="2"></v-divider>
                <v-list-item link class="mt-2" v-for="(contact, i) in contacts" :title="contact.name"
                    @click="messageState.activeContact = contact.id"
                    :variant="messageState.activeContact === contact.id ? 'tonal' : 'plain'" :key="i">
                    <template v-slot:prepend>
                        <v-list-item-icon class="mr-5">
                            <v-icon>mdi-account</v-icon>
                            <v-icon style="position: absolute; top: 3px; left: 3px;"
                                :style="`color: ${statuses.find(stat => stat.status === contact.status)?.color ?? 'grey'}`">mdi-circle-medium</v-icon>
                        </v-list-item-icon>
                    </template>
                    <v-list-item-content>
                        <v-list-item-subtitle>{{ contact.messages[contact.messages.length - 1].message
                        }}</v-list-item-subtitle>
                    </v-list-item-content>
                </v-list-item>
            </div>
        </v-col>
        <v-col>
            <div v-if="currentContact !== undefined">
                <v-toolbar>
                    <template v-slot:prepend>
                        <v-icon>mdi-account</v-icon>
                        <v-icon style="position: absolute; top: 10px; left: 0px;"
                            :style="`color: ${statuses.find(stat => stat.status === currentContact?.status)?.color ?? 'grey'}`">mdi-circle-medium</v-icon>
                    </template>
                    <v-toolbar-title>{{ currentContact.name }}</v-toolbar-title>
                    <v-spacer></v-spacer>
                    <v-icon>mdi-dots-vertical</v-icon>
                </v-toolbar>
                <v-card height="80vh" flat>
                    <v-card-text class="flex-grow-1 overflow-y-auto">
                        <div v-for="(item, i) in currentContact.messages" :key="i" class="message-wrapper">
                            <div class="message"
                                :class="item.from === 'Me' ? 'bg-primary message-sent' : 'bg-secondary message-recieved'">
                                <p>
                                    {{ item.message }}
                                </p>
                            </div>
                        </div>
                    </v-card-text>
                    <v-footer>
                        <v-textarea v-model="messageState.contactDraftMessages[currentContact.id]"
                            style="width: 75%; align-self: flex-end;" rows="1" auto-grow>
                            <template v-slot:append-inner>
                                <v-btn variant="text" icon="mdi-send"></v-btn>
                            </template>
                        </v-textarea>
                    </v-footer>
                </v-card>

            </div>
        </v-col>
    </v-row>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue';
import { useAppStore } from '../store/app';
import { toRefs } from 'vue';
import { useDisplay } from 'vuetify';
const { xs } = useDisplay();
const { messages: messageState } = toRefs(useAppStore());
const userStatus = ref("online");
const smAndNavCollapsed = ref(xs.value);

const statuses = [
    { status: "online", color: "green" },
    { status: "away", color: "orange" },
    { status: "offline", color: "grey" },
]
const contacts = [
    {
        id: "1", name: "John Doe", icon: "", status: "online", messages: [
            { from: "John Doe", message: "Hello", time: "10:00" },
            { from: "Me", message: "Hi", time: "10:01" },
            { from: "John Doe", message: "How are you?", time: "10:02" },
            { from: "Me", message: "I'm fine, thank you", time: "10:03" },
            { from: "John Doe", message: "What are you doing?", time: "10:04" },
            { from: "Me", message: "I'm working", time: "10:05" },
            { from: "John Doe", message: "Ok", time: "10:06" },
            { from: "John Doe", message: "Hello", time: "10:00" },
            { from: "Me", message: "Hi", time: "10:01" },
            { from: "John Doe", message: "How are you?", time: "10:02" },
            { from: "Me", message: "I'm fine, thank you", time: "10:03" },
            { from: "John Doe", message: "What are you doing?", time: "10:04" },
            { from: "Me", message: "I'm working", time: "10:05" },
            { from: "John Doe", message: "Ok", time: "10:06" },
        ]
    },
    {
        id: "2", name: "Jane Doe", icon: "", status: "away", messages: [
            { from: "Jane Doe", message: "Hello", time: "10:00" },
            { from: "Me", message: "Hi", time: "10:01" },
            { from: "Jane Doe", message: "How are you?", time: "10:02" },
            { from: "Me", message: "I'm fine, thank you", time: "10:03" },
            { from: "Jane Doe", message: "What are you doing?", time: "10:04" },
            { from: "Me", message: "I'm working", time: "10:05" },
            { from: "Jane Doe", message: "Ok", time: "10:06" },
        ]
    },
    {
        id: "3", name: "John Smith", icon: "", status: "offline", messages: [
            { from: "John Smith", message: "Hello", time: "10:00" },
            { from: "Me", message: "Hi", time: "10:01" },
            { from: "John Smith", message: "How are you?", time: "10:02" },
            { from: "Me", message: "I'm fine, thank you", time: "10:03" },
            { from: "John Smith", message: "What are you doing?", time: "10:04" },
            { from: "Me", message: "I'm working", time: "10:05" },
            { from: "John Smith", message: "Ok", time: "10:06" },
        ]
    }
]
const currentContact = computed(() => {
    return contacts.find(contact => contact.id === messageState.value.activeContact);
})
</script>

<style scoped>
html {
    overflow: hidden !important;
}

.v-card {
    display: flex !important;
    flex-direction: column;
}

.v-card__text {
    flex-grow: 1;
    overflow: auto;
}

.message-wrapper {
    display: flex;
    flex-direction: column;
}

.message {
    padding: 10px;
    margin: 10px;
    border-top-right-radius: 10px;
    border-top-left-radius: 10px;
    margin-bottom: 20px;
}

.message-sent {
    border-bottom-left-radius: 10px;
    align-self: flex-end;
}

.message-recieved {
    border-bottom-right-radius: 10px;
    align-self: flex-start;
}
</style>